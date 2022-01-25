use actix_web::{HttpResponse, web::{Data, Json}};
use bundlr_sdk::{deep_hash::{deep_hash, DeepHashChunk, ONE_AS_BUFFER}, JWK, deep_hash_sync::deep_hash_sync};
use bytes::Bytes;
use data_encoding::{BASE64URL, BASE64URL_NOPAD};
use diesel::RunQueryDsl;
use jsonwebkey::JsonWebKey;
use lazy_static::lazy_static;
use openssl::{sign, hash::MessageDigest, rsa::Padding, pkey::{PKey, Private, Public}};
use redis::AsyncCommands;
use reool::{RedisPool, PoolDefault};
use serde::{Serialize, Deserialize};

use crate::{server::error::ValidatorServerError, types::DbPool, database::{schema::transactions::dsl::*, models::{Transaction, NewTransaction}}, consts::{BUNDLR_AS_BUFFER, VALIDATOR_AS_BUFFER}};

#[derive(Deserialize)]
pub struct UnsignedBody {
    id: String,
    signature: String,
    block: u128
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedBody {
    id: String,
    signature: String,
    block: u128,
    validator_address: String,
    validator_signature: String
}

lazy_static! {
    static ref BUNDLER_PUBLIC: Vec<u8> = {
        let var = std::env::var("BUNDLER_PUBLIC").unwrap();
        var.as_bytes().to_vec()
    };
    static ref BUNDLER_ADDRESS: String = BASE64URL.encode(std::env::var("BUNDLER_PUBLIC").unwrap().as_bytes());
}

pub async fn sign_route(db: Data<DbPool>, redis: Data<RedisPool>, body: Json<UnsignedBody>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let body = body.into_inner();

    let mut conn = redis.check_out(PoolDefault)
        .await
        .unwrap();

    // Verify
    if conn.exists(&body.id).await.unwrap() { return Ok(HttpResponse::Accepted().finish()); };
    let current_block = conn.get::<_, u128>(&body.id).await.unwrap();

    if body.block < (current_block - 5) || body.block > (current_block + 5) {
        return Ok(HttpResponse::BadRequest().finish());
    }
    
    if !verify_body(&body) {
        return Ok(HttpResponse::BadRequest().finish());
    };

    // Sign
    let sig = sign_body(body.id.as_str(), BUNDLER_ADDRESS.as_str())
        .await;

    // Add to db
    let current_epoch = conn.get::<_, i64>("validator:epoch:current")
        .await
        .unwrap();
        
    let new_transaction = NewTransaction {
        id: body.id,
        epoch: current_epoch,
        block_promised: i64::try_from(body.block).unwrap(),
        block_actual: None,
        signature: sig.clone(),
        validated: false,
    };

    actix_rt::task::spawn_blocking(move || {
        let c = db.get().unwrap();
        diesel::insert_into(transactions)
            .values::<NewTransaction>(new_transaction)
            .execute(&c)
    }).await??;
   

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .body(sig))
}

fn verify_body(body: &UnsignedBody) -> bool {
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

    dbg!(message.to_vec());


    lazy_static! {
        static ref PUBLIC: PKey<Public> = {
            let jwk = JWK {
                kty: "RSA",
                e: "AQAB",
                n: std::env::var("BUNDLER_PUBLIC").unwrap()
            };

            let p = serde_json::to_string(&jwk).unwrap();
            let key: JsonWebKey = p.parse().unwrap();
            
            PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap()
        };
    };

    let sig = BASE64URL_NOPAD.decode(body.signature.as_bytes()).unwrap();
    
    dbg!(sig.clone());


    let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &PUBLIC).unwrap();
    verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    verifier.update(&message).unwrap();
    verifier.verify(&sig).map_err(|e| { dbg!(e); false }).unwrap()
}

async fn sign_body(tx_id: &str, bundler_address: &str) -> Vec<u8> {
    let message = deep_hash(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(VALIDATOR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
        DeepHashChunk::Chunk(BASE64URL_NOPAD.decode(tx_id.as_bytes()).unwrap().into()),
        DeepHashChunk::Chunk(BASE64URL_NOPAD.decode(bundler_address.as_bytes()).unwrap().into())
    ]))
        .await.unwrap();

    lazy_static! {
        static ref KEY: PKey<Private> = {
            let file: String = String::from_utf8(include_bytes!("../../../wallet.json").to_vec()).unwrap();
            let key: JsonWebKey = file.parse().unwrap();
            let pem = key.key.to_pem();
            PKey::private_key_from_pem(pem.as_bytes()).unwrap()
        };
    };

    let mut signer = sign::Signer::new(MessageDigest::sha256(), &KEY).unwrap();
    signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    signer.update(&message).unwrap();
    let mut sig = vec![0;256];
    signer.sign(&mut sig).unwrap();

    sig
}


#[cfg(test)]
mod tests {
    use super::verify_body;
    use super::UnsignedBody;

    #[test]
    fn test_sign_and_verify() {
        dotenv::dotenv();

        let body = UnsignedBody {
            id: "dtdOmHZMOtGb2C0zLqLBUABrONDZ5rzRh9NengT1-Zk".into(),
            signature: "0QaA-U51cTPGqKgxqiN0-0NYB-P09uHaReR4U9iZwmYNgx7u9DQbDVYrunmUChAy8IAlan4Qi7mKKFKHk3QAzR71MggMQ7nsp9tvk-OYnytFiVoiI1xRuKKFT_86kb5Eq5Oj-7vgM6XR6Giv44-33Ma1MEpuvW6FEEkfqPpdritaNgmBuv6GsEL3CGqutC5pOW1eBev3i3VSVbICfcHUgKvbklPOI5k2eez3_K3bC1qOeXl3Twr3yEPuSqZjaW5Xo5F81rznvsrxj93yT53kTAb-70EZVBWSqKIh8-JClMYjMQ3xKhrNDEYlR4lXapn3FSP2wkMkbCSay53u-rdQADqh-bjwg5Jf4JwULTFS10cRBiYpG7fHFztjzRVmnA4aUrpkzLUwjOldYFNr3z48h14l4hIoGZTK6z87_ycsPaKGocCP_qCJfr0o8FaYgXNKMIU_uNeuGqZ0Qr_iebnu3CQUkpWgFNwE-WxnQDOYDomMXDPkCYzehJiCEdWcQOwOlGHbMBLngDmVO0r6ZYbea3e9ahp5NoBxbP9xbY5Vsdmt-ENrt2bdfTz4Ek4rKQ_x0xKdO3nO-sLfONyqX1BcFTqXsK_X-SgVMVUh2ObzlBKEUOEle_9NfIptidVVkiiuiDrnR7_Tgq0RqLP5VbHYsYCNfMVnM33GP1cabgcuquM".into(),
            block: 500,
        };

        assert!(verify_body(&body));
    }
}