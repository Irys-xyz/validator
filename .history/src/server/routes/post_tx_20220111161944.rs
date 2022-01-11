use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{HttpResponse, web::{Json, Data}};
use bundlr_sdk::{deep_hash::{deep_hash, deep_hash_chunks, DeepHashChunk}, deep_hash_sync::deep_hash_sync, JWK};
use bytes::Bytes;
use jsonwebkey::JsonWebKey;
use openssl::{sign::Verifier, hash::MessageDigest, rsa::{Padding}, pkey::PKey};
use redis::{AsyncCommands};
use serde::{Deserialize, Serialize};
use data_encoding::BASE64URL;
use crate::server::error::ValidatorServerError;


#[derive(Serialize, Deserialize)]
pub struct ValidatorSignature {
    public: String,
    signature: String
}

#[derive(Serialize, Deserialize)]
pub struct PostTxBody {
    id: String,
    signature: String,
    block: u128,
    address: String,
    #[serde(default)]
    validator_signatures: Vec<ValidatorSignature>
}

// Receive Bundlr transaction receipt
pub async fn post_tx(body: Json<PostTxBody>, redis_client: Data<redis::Client>, awc_client: Data<awc::Client>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
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

    let number_of_sigs = body.validator_signatures.len();

    let valid = actix_rt::task::spawn_blocking(move || {
        let jwk = JWK {
            kty: "RSA",
            e: "AQAB",
            n: BASE64URL.encode(public.as_bytes())
        };

        let p = serde_json::to_string(&jwk).unwrap();
        let key: JsonWebKey = p.parse().unwrap();
        
        let pkey = PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap();   

        let body_string = serde_json::to_string(&body).unwrap();

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

    if number_of_sigs < 3 {
        let mut response = awc_client
            .post(format!(""))
            .send_json()
            .await
            .unwrap();
    }

    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .unwrap();
    
    let in_ms = since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    
    // Add to db
    conn.set(&key, in_ms).await?;
    conn.expire(&key, 172800).await?;

    Ok(HttpResponse::Ok().finish())
}

// TODO: Fix this
fn deep_hash_body(body: &PostTxBody) -> Result<Bytes, ValidatorServerError> {
    deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(body.id.clone().into()),
        DeepHashChunk::Chunk(body.block.to_string().into())
    ])).map_err(|_| ValidatorServerError::InternalError)
}