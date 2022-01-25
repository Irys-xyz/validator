use std::sync::RwLock;

use actix_web::{HttpResponse, web::{Json, Data}};
use bundlr_sdk::{deep_hash::{DeepHashChunk, ONE_AS_BUFFER}, deep_hash_sync::deep_hash_sync, JWK};
use bytes::Bytes;
use jsonwebkey::JsonWebKey;
use openssl::{sign::{Verifier, self}, hash::MessageDigest, rsa::{Padding}, pkey::{PKey, Public}};
use redis::{AsyncCommands};
use serde::{Deserialize, Serialize};
use data_encoding::BASE64URL;
use crate::{server::error::ValidatorServerError, consts::BUNDLR_AS_BUFFER};
use lazy_static::lazy_static;

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidatorSignature {
    public: String,
    signature: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PostTxBody {
    id: String,
    signature: String,
    block: u128,
    address: String,
    #[serde(default)]
    validator_signatures: Vec<ValidatorSignature>
}

// Receive Bundlr transaction receipt
pub async fn post_tx(
    body: Json<PostTxBody>,
    redis_client: Data<redis::Client>, 
    awc_client: Data<awc::Client>,
    validators: Data<RwLock<Vec<String>>>
) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let validators = validators.into_inner();
    let body = body.into_inner();
    let mut conn = redis_client.get_async_connection().await?;

    let key = format!("validator:tx:{}", body.id);

    if conn.exists(&key).await? {
        return Ok(HttpResponse::Accepted().finish());
    };

    // Check address is valid

    // Get public
    let public = match conn.get::<_, String>(format!("validator:bundler:{}:public", body.address)).await {
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
            n: BASE64URL.encode(public.as_bytes())
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
    let block = body.block.to_string()
        .as_bytes()
        .to_vec();

    let tx_id = body.id.as_bytes().to_vec();

    let message = deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
        DeepHashChunk::Chunk(tx_id.into()),
        DeepHashChunk::Chunk(block.into())
    ])).unwrap();


    lazy_static! {
        static ref PUBLIC: PKey<Public> = {
            let jwk = JWK {
                kty: "RSA",
                e: "AQAB",
                n: BASE64URL.encode(std::env::var("BUNDLER_PUBLIC").unwrap().as_bytes())
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
    if !verifier.verify(&sig).unwrap_or(false)
}

// TODO: Fix this
fn deep_hash_body(body: &PostTxBody) -> Result<Bytes, ValidatorServerError> {
    deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(body.id.clone().into()),
        DeepHashChunk::Chunk(body.block.to_string().into())
    ])).map_err(|_| ValidatorServerError::InternalError)
}

async fn add_to_db(body: &PostTxBody) -> Result<(), ValidatorServerError> {
    Ok(())
}