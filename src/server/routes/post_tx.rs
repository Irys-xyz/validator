use std::{
    collections::HashMap,
    sync::{atomic::Ordering, RwLock},
};

use crate::{
    consts::BUNDLR_AS_BUFFER,
    database::schema::{transactions, transactions::dsl::*},
    server::error::ValidatorServerError,
    state::{ValidatorState, ValidatorStateTrait},
    types::DbPool,
};
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
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use jsonwebkey::JsonWebKey;
use lazy_static::lazy_static;
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Public},
    rsa::Padding,
    sha::sha256,
    sign::{self, Verifier},
};
use paris::error;
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
pub async fn post_tx<Config>(
    ctx: Data<Config>,
    body: Json<PostTxBody>,
    db: Data<DbPool>,
    _awc_client: Data<awc::Client>,
    validators: Data<RwLock<Vec<String>>>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Config: super::sign::Config + 'static,
{
    let s = ctx.get_validator_state().load(Ordering::SeqCst);
    if s != ValidatorState::Leader {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let _validators = validators.into_inner();
    let body = body.into_inner();

    let key = format!("validator:tx:{}", body.id);

    let exists = {
        let conn = db.get().map_err(|err| {
            error!("Failed to get database connection: {:?}", err);
            ValidatorServerError::InternalError
        })?;
        let filter = id.eq(body.id.clone());
        actix_rt::task::spawn_blocking(move || {
            match transactions.filter(filter).count().get_result(&conn) {
                Ok(0) => Ok(false),
                Ok(_) => Ok(true),
                Err(err) => Err(err),
            }
        })
    };

    if let Ok(true) = exists
        .await
        .map_err(|_| ValidatorServerError::InternalError)?
    {
        return Ok(HttpResponse::Accepted().finish());
    }

    // Check address is valid

    // Get public
    // FIXME: remove crypto operations behind a service
    // Instead of passing public and private keys around,
    // create service that allows signing messages and verifying
    // signatures. This way, key management can be in one place
    // and reference to this service can be passed more easily
    // around to async tasks.
    let public = ctx.bundler_public_key().clone();

    let (valid, body) = actix_rt::task::spawn_blocking(move || {
        let hash = deep_hash_body(&body).unwrap();

        // Check signature matches public
        let mut verifier = Verifier::new(MessageDigest::sha256(), &public)?;
        verifier.set_rsa_padding(Padding::PKCS1_PSS)?;
        verifier.update(&hash)?;

        // FIXME: Assumes sig is base64url
        let sig = BASE64URL.decode(body.signature.as_bytes()).unwrap();

        verifier
            .verify(sig.as_slice())
            .map(|verified| (verified, body))
    })
    .await??;

    if !valid {
        tracing::info!("Received invalid signature");
        return Ok(HttpResponse::BadRequest().finish());
    };

    add_to_db(&body).await?;

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
