use actix_web::{HttpResponse, web::{Json, Data}};
use openssl::{sign::Verifier, hash::MessageDigest, rsa::{Padding}, pkey::PKey};
use redis::{AsyncCommands};
use serde::{Deserialize, Serialize};
use data_encoding::BASE64URL;
use crate::server::error::ValidatorServerError;

#[derive(Serialize, Deserialize)]
pub struct PostTxBody {
    id: String,
    signature: String,
    block: u128,
    address: String
}


pub async fn post_tx(body: Json<PostTxBody>, redis_client: Data<redis::Client>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let body = body.into_inner();
    let mut conn = redis_client.get_async_connection().await?;

    let key = format!("validator:tx:{}", body.id);
    // Check id doesn't already exists
    let exists = conn.exists(&key).await?;

    if exists {
        return Ok(HttpResponse::Accepted().finish());
    };

    // Check address is valid

    // Get public
    let public = match conn.get::<_, String>(format!("validator:bundler:{}:public", body.address)).await {
        Ok(n) => {
            match BASE64URL.decode(n.as_bytes()) {
                Ok(decoded) => decoded,
                Err(e) => {
                    tracing::error!("Error occurred while decoding bundler public - {}", e);
                    return Ok(HttpResponse::BadRequest().finish());
                }
            }
        },
        Err(e) => {
            tracing::error!("Error occurred while getting bundler public - {}", e);
            return Ok(HttpResponse::BadRequest().finish());
        }
    };
    let pkey = PKey::public_key_from_der(public.as_slice())?;

    let body_string = serde_json::to_string(&body).unwrap();

    let valid = actix_rt::task::spawn_blocking(move || {
        // Check signature matches public
        let hash = deep_hash_body(&body);
            
        let mut verifier = Verifier::new(MessageDigest::sha256(), &pkey)?;
        verifier.set_rsa_padding(Padding::PKCS1_PSS)?;
        verifier.update(hash)?;

        // FIXME: Assumes sig is base64url
        let sig = BASE64URL.decode(&body.signature.as_bytes()).unwrap();

        verifier.verify(sig.as_slice())
    })
    .await??;

    if !valid {
        tracing::info!("Received invalid signature");
        return Ok(HttpResponse::BadRequest().finish());
    };

    // Add to db
    conn.set(&key, body_string).await?;
    conn.expire(&key, 172800).await?;

    Ok(HttpResponse::Ok().finish())
}

fn deep_hash_body(body: &PostTxBody) -> &[u8] {
    return &[1];
}