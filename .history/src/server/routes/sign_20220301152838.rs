use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use bundlr_sdk::{
    deep_hash::{deep_hash, DeepHashChunk, ONE_AS_BUFFER},
    deep_hash_sync::deep_hash_sync,
};

use data_encoding::BASE64URL_NOPAD;
use diesel::RunQueryDsl;
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private, Public},
    rsa::Padding,
    sign,
};
use redis::AsyncCommands;
use reool::{PoolDefault, RedisPool};
use serde::{Deserialize, Serialize};

use crate::{
    consts::{BUNDLR_AS_BUFFER, VALIDATOR_AS_BUFFER},
    database::{
        models::{NewTransaction, Transaction},
        schema::transactions::dsl::*,
    },
    server::error::ValidatorServerError,
    types::DbPool,
};

pub trait Config {
    fn bundler_address(&self) -> &str;
    fn bundler_public_key(&self) -> &PKey<Public>;
    fn validator_address(&self) -> &str;
    fn validator_private_key(&self) -> &PKey<Private>;
    fn validator_public_key(&self) -> &PKey<Public>;
}

#[derive(Deserialize)]
pub struct UnsignedBody {
    id: String,
    signature: String,
    block: u128,
    validators: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedBody {
    id: String,
    signature: String,
    block: u128,
    validator_address: String,
    validator_signature: String,
}

pub async fn sign_route<Config>(
    db: Data<DbPool>,
    redis: Data<RedisPool>,
    body: Json<UnsignedBody>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Config: self::Config,
{
    let body = body.into_inner();

    let config: Config = todo!();

    let mut conn = redis.check_out(PoolDefault).await.unwrap();

    // Verify
    if conn.exists(&body.id).await.unwrap() {
        return Ok(HttpResponse::Accepted().finish());
    };
    let current_block = conn.get::<_, u128>(&body.id).await.unwrap();

    if !body
        .validators
        .contains(&config.validator_address().to_string())
    {
        return Ok(HttpResponse::BadRequest().finish());
    }

    if body.block < (current_block - 5) || body.block > (current_block + 5) {
        return Ok(HttpResponse::BadRequest().finish());
    }

    if !verify_body(config.bundler_public_key(), &body) {
        return Ok(HttpResponse::BadRequest().finish());
    };

    // Sign
    let sig = sign_body(
        config.validator_private_key(),
        config.bundler_address(),
        body.id.as_str(),
    )
    .await;

    // Add to db
    let current_epoch = conn.get::<_, i64>("validator:epoch:current").await.unwrap();

    let new_transaction = NewTransaction {
        id: body.id,
        epoch: current_epoch,
        block_promised: i64::try_from(body.block).unwrap(),
        block_actual: None,
        signature: sig.clone(),
        validated: false,
        bundle_id: None,
        sent_to_leader: false,
    };

    actix_rt::task::spawn_blocking(move || {
        let c = db.get().unwrap();
        diesel::insert_into(transactions)
            .values::<NewTransaction>(new_transaction)
            .execute(&c)
    })
    .await??;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .body(sig))
}

fn verify_body(bundler_key: &PKey<Public>, body: &UnsignedBody) -> bool {
    let block = body.block.to_string().as_bytes().to_vec();
    let tx_id = body.id.as_bytes().to_vec();

    let message = deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
        DeepHashChunk::Chunk(tx_id.into()),
        DeepHashChunk::Chunk(block.into()),
    ]))
    .unwrap();

    let sig = BASE64URL_NOPAD.decode(body.signature.as_bytes()).unwrap();

    let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &bundler_key).unwrap();
    verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    verifier.update(&message).unwrap();
    // TODO: we shouldn't probably hide errors here, at least we should log them
    verifier.verify(&sig).unwrap_or(false)
}

async fn sign_body(validator_key: &PKey<Private>, bundler_address: &str, tx_id: &str) -> Vec<u8> {
    let tx_id = tx_id.as_bytes().to_vec();

    let message = deep_hash(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(VALIDATOR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
        DeepHashChunk::Chunk(tx_id.into()),
        DeepHashChunk::Chunk(bundler_address.to_string().into()),
    ]))
    .await
    .unwrap();

    dbg!(message.clone());

    let mut signer = sign::Signer::new(MessageDigest::sha256(), &validator_key).unwrap();
    signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    signer.update(&message).unwrap();
    let mut sig = vec![0; 512];
    signer.sign(&mut sig).unwrap();

    sig
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use bundlr_sdk::deep_hash::{DeepHashChunk, ONE_AS_BUFFER};
    use bundlr_sdk::deep_hash_sync::deep_hash_sync;
    use data_encoding::BASE64URL_NOPAD;
    use jsonwebkey::{JsonWebKey, Key, PublicExponent, RsaPrivate, RsaPublic};
    use openssl::hash::MessageDigest;
    use openssl::pkey::{PKey, Private, Public};
    use openssl::rsa::{Padding, Rsa};
    use openssl::sha::Sha256;
    use openssl::sign::{self, Signer, Verifier};

    use crate::consts::{BUNDLR_AS_BUFFER, VALIDATOR_AS_BUFFER};

    use super::UnsignedBody;
    use super::{sign_body, verify_body};

    fn bundler_key() -> (JsonWebKey, PKey<Private>) {
        let rsa = Rsa::generate(2048).unwrap();
        let n = rsa.n().to_vec().into();

        let pkey = PKey::from_rsa(rsa).unwrap();
        let private_der = pkey.private_key_to_der().unwrap();

        (
            JsonWebKey::new(Key::RSA {
                public: RsaPublic {
                    e: PublicExponent,
                    n,
                },
                private: None,
            }),
            PKey::private_key_from_der(&private_der.as_slice()).unwrap(),
        )
    }

    fn validator_key() -> JsonWebKey {
        let rsa = Rsa::generate(2048).unwrap();

        JsonWebKey::new(Key::RSA {
            public: RsaPublic {
                e: PublicExponent,
                n: rsa.n().to_vec().into(),
            },
            private: Some(RsaPrivate {
                d: rsa.d().to_vec().into(),
                p: rsa.p().map(|v| v.to_vec().into()),
                q: rsa.q().map(|v| v.to_vec().into()),
                dp: rsa.dmp1().map(|v| v.to_vec().into()),
                dq: rsa.dmq1().map(|v| v.to_vec().into()),
                qi: rsa.iqmp().map(|v| v.to_vec().into()),
            }),
        })
    }

    fn to_private_key(key: &JsonWebKey) -> Result<PKey<Private>, ()> {
        let der: Vec<u8> = key.key.try_to_der().map_err(|err| {
            eprintln!("Failed to extract der: {:?}", err);
            ()
        })?;
        PKey::private_key_from_der(der.as_slice()).map_err(|err| {
            eprintln!("Failed to extract public key from der: {:?}", err);
            ()
        })
    }

    fn to_public_key(jwk: &JsonWebKey) -> Result<PKey<Public>, ()> {
        let der = if jwk.key.is_private() {
            let pub_key = jwk.key.to_public().ok_or_else(|| {
                eprintln!("Key has no public part");
            })?;
            pub_key.try_to_der().map_err(|err| {
                eprintln!("Failed to extract der: {:?}", err);
                ()
            })?
        } else {
            jwk.key.try_to_der().map_err(|err| {
                eprintln!("Failed to extract der: {:?}", err);
                ()
            })?
        };
        PKey::public_key_from_der(der.as_slice()).map_err(|err| {
            eprintln!("Failed to extract public key from der: {:?}", err);
            ()
        })
    }

    fn to_address(key: &JsonWebKey) -> Result<String, ()> {
        let pub_key: PKey<Public> = to_public_key(key)?;
        let mut hasher = Sha256::new();
        hasher.update(&pub_key.rsa().unwrap().n().to_vec());
        let hash = hasher.finish();
        Ok(BASE64URL_NOPAD.encode(&hash))
    }

    fn test_message(signing_key: &PKey<Private>) -> UnsignedBody {
        let tx_id = "dtdOmHZMOtGb2C0zLqLBUABrONDZ5rzRh9NengT1-Zk";
        let block = 500;
        let message = deep_hash_sync(DeepHashChunk::Chunks(vec![
            DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
            DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
            DeepHashChunk::Chunk(tx_id.into()),
            DeepHashChunk::Chunk(format!("{}", block).into()),
        ]))
        .unwrap();

        let (buf, len) = {
            let mut signer = sign::Signer::new(MessageDigest::sha256(), &signing_key).unwrap();
            signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
            signer.update(&message).unwrap();
            let mut buf = vec![0; 512];
            let len = signer.sign(&mut buf).unwrap();
            (buf, len)
        };

        let sig = BASE64URL_NOPAD.encode(&buf[0..len]);

        UnsignedBody {
            id: tx_id.to_string(),
            signature: sig,
            block,
            validators: vec![],
        }
    }

    #[test]
    fn get_public_key_from_public_key_only_jwk() {
        let (jwk, _) = bundler_key();

        let pub_key = to_public_key(&jwk);
        assert!(pub_key.is_ok());
    }

    #[test]
    fn get_public_key_from_private_key_containing_jwk() {
        let jwk = validator_key();

        let pub_key = to_public_key(&jwk);
        assert!(pub_key.is_ok());
    }

    #[test]
    fn get_private_key_from_private_key_containing_jwk() {
        let jwk = validator_key();

        let key = to_private_key(&jwk);
        assert!(key.is_ok());
    }

    #[test]
    fn get_private_key_from_public_key_only_jwk() {
        let (jwk, _) = bundler_key();

        let key = to_private_key(&jwk);
        assert!(key.is_err());
    }

    #[test]
    fn test_verify_body() {
        let (bundler_key, signing_key) = bundler_key();
        let body = test_message(&signing_key);
        let key = to_public_key(&bundler_key).unwrap();
        assert!(verify_body(&key, &body));
    }

    #[actix_rt::test]
    async fn test_sign_body() {
        let (bundler_jwk, bundler_signing_key) = bundler_key();
        let validator_jwk = validator_key();
        let body = test_message(&bundler_signing_key);
        let sig = sign_body(
            &to_private_key(&validator_jwk).unwrap(),
            &to_address(&bundler_jwk).unwrap(),
            &body.id,
        )
        .await;

        let tx_id = body.id.as_bytes().to_vec();
        let bundler_address = to_address(&bundler_jwk).unwrap();
        let message = deep_hash_sync(DeepHashChunk::Chunks(vec![
            DeepHashChunk::Chunk(VALIDATOR_AS_BUFFER.into()),
            DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
            DeepHashChunk::Chunk(tx_id.into()),
            DeepHashChunk::Chunk(bundler_address.into()),
        ]))
        .unwrap();

        dbg!(message.clone());

        let validator_key = to_public_key(&validator_jwk).unwrap();

        let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &validator_key).unwrap();
        verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
        verifier.update(&message).unwrap();
        let verified = verifier.verify(&sig).unwrap();
        assert!(verified)
    }
}
