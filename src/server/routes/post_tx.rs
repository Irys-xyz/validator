use std::{collections::HashMap, sync::RwLock};

use crate::{consts::BUNDLR_AS_BUFFER, server::error::ValidatorServerError};
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use bundlr_sdk::{
    deep_hash::{DeepHashChunk, ONE_AS_BUFFER},
    deep_hash_sync::deep_hash_sync,
    JWK,
};
use bytes::Bytes;
use data_encoding::BASE64URL;
use jsonwebkey::JsonWebKey;
use lazy_static::lazy_static;
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Public},
    rsa::Padding,
    sha::sha256,
    sign::{self, Verifier},
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidatorSignature {
    public: String,
    signature: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PostTxBody {
    id: String,
    signature: String,
    block: i64,
    address: String,
    #[serde(default)]
    validator_signatures: Vec<ValidatorSignature>,
}

// Receive Bundlr transaction receipt
pub async fn post_tx(
    body: Json<PostTxBody>,
    redis_client: Data<redis::Client>,
    _awc_client: Data<awc::Client>,
    validators: Data<RwLock<Vec<String>>>,
) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let _validators = validators.into_inner();
    let body = body.into_inner();
    let mut conn = redis_client.get_async_connection().await?;

    let key = format!("validator:tx:{}", body.id);

    if conn.exists(&key).await? {
        return Ok(HttpResponse::Accepted().finish());
    };

    // Check address is valid

    // Get public
    let public = match conn
        .get::<_, String>(format!("validator:bundler:{}:public", body.address))
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Error occurred while getting bundler public - {}", e);
            return Ok(HttpResponse::BadRequest().finish());
        }
    };

    let body_clone = body.clone();

    let valid = actix_rt::task::spawn_blocking(move || {
        let jwk = JWK {
            kty: "RSA",
            e: "AQAB",
            n: BASE64URL.encode(public.as_bytes()),
        };

        let p = serde_json::to_string(&jwk).unwrap();
        let key: JsonWebKey = p.parse().unwrap();

        let pkey = PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap();

        let hash = deep_hash_body(&body).unwrap();

        // Check signature matches public
        let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey)?;
        verifier.set_rsa_padding(Padding::PKCS1_PSS)?;
        verifier.update(&hash)?;

        // FIXME: Assumes sig is base64url
        let sig = BASE64URL.decode(body.signature.as_bytes()).unwrap();

        verifier.verify(sig.as_slice())
    })
    .await??;

    if !valid {
        tracing::info!("Received invalid signature");
        return Ok(HttpResponse::BadRequest().finish());
    };

    add_to_db(&body_clone).await?;

    Ok(HttpResponse::Ok().finish())
}

fn verify_body(body: &PostTxBody) -> bool {
    if body.validator_signatures.len() < 3 {
        return false;
    };

    let block = body.block.to_string().as_bytes().to_vec();

    let tx_id = body.id.as_bytes().to_vec();

    let message = deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
        DeepHashChunk::Chunk(tx_id.into()),
        DeepHashChunk::Chunk(block.into()),
    ]))
    .unwrap();

    lazy_static! {
        static ref PUBLIC: PKey<Public> = {
            let jwk = JWK {
                kty: "RSA",
                e: "AQAB",
                n: BASE64URL.encode(std::env::var("BUNDLER_PUBLIC").unwrap().as_bytes()),
            };

            let p = serde_json::to_string(&jwk).unwrap();
            let key: JsonWebKey = p.parse().unwrap();

            PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap()
        };
    };

    let sig = BASE64URL.decode(body.signature.as_bytes()).unwrap();

    let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &PUBLIC).unwrap();
    verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    verifier.update(&message).unwrap();
    if !verifier.verify(&sig).unwrap_or(false) {
        return false;
    };

    let validators_in_epoch = HashMap::<String, String>::new();
    body.validator_signatures.iter().all(|sig| {
        let address = public_to_address(&sig.public);
        if !validators_in_epoch.contains_key(&address) {
            return false;
        };

        true
    })
}

// TODO: Fix this
fn deep_hash_body(body: &PostTxBody) -> Result<Bytes, ValidatorServerError> {
    deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(body.id.clone().into()),
        DeepHashChunk::Chunk(body.block.to_string().into()),
    ]))
    .map_err(|_| ValidatorServerError::InternalError)
}

async fn add_to_db(_body: &PostTxBody) -> Result<(), ValidatorServerError> {
    Ok(())
}

#[warn(dead_code)]
fn public_to_address(n: &str) -> String {
    BASE64URL.encode(&sha256(&BASE64URL.decode(n.as_bytes()).unwrap())[..])
}
