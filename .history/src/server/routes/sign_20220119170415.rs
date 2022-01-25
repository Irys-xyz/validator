use core::lazy;

use actix_web::{HttpResponse, web::{Data, Json}};
use bundlr_sdk::{deep_hash::{deep_hash, DeepHashChunk}, JWK};
use awc::ws::Message;
use bytes::Bytes;
use data_encoding::BASE64URL;
use diesel::RunQueryDsl;
use jsonwebkey::JsonWebKey;
use lazy_static::lazy_static;
use openssl::{sign, hash::MessageDigest, rsa::Padding, pkey::{PKey, Private, Public}};
use redis::AsyncCommands;
use reool::{RedisPool, PoolDefault};
use serde::{Serialize, Deserialize};

use crate::{server::error::ValidatorServerError, types::DbPool, database::{schema::transactions::dsl::*, models::{Transaction, NewTransaction}}};

#[derive(Deserialize)]
pub struct UnsignedBody {
    id: String,
    address:String,
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

pub async fn sign(db: Data<DbPool>, redis: Data<RedisPool>, body: Json<UnsignedBody>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let body = body.into_inner();

    let mut conn = redis.check_out(PoolDefault)
        .await
        .unwrap();

    // Verify
    if conn.exists(&body.id).await.unwrap() {
        return Ok(HttpResponse::Accepted().finish());
    };

    let decoded_sig = BASE64URL.decode(body.signature.as_bytes()).unwrap();
    
    if !verify_body(&body.id, &body.address, &decoded_sig).await {
        return Ok(HttpResponse::BadRequest().finish());
    };

    // Sign
    let sig = sign_body(body.id.as_str(), body.address.as_str())
        .await;

    // Add to db
    let current_epoch = conn.get::<_, i64>("validator:epoch:current")
        .await
        .unwrap();
        
    let new_transaction = NewTransaction {
        id: body.id.as_str(),
        bundler: body.address.as_str(),
        epoch: current_epoch,
        block_promised: i64::try_from(body.block).unwrap(),
        block_actual: None,
        signature: &sig,
        validated: false,
    };

    let conn = db.get().unwrap();

    diesel::insert_into(transactions)
        .values::<NewTransaction>(new_transaction)
        .execute(&conn)
        .unwrap();

    Ok(HttpResponse::Ok()
    .insert_header(("Content-Type", "application/octet-stream"))
    .body(sig))
}

const BUNDLR_AS_BUFFER: &[u8] = "Bundlr".as_bytes();

async fn verify_body(tx_id: &str, bundler_address: &str, sig: &[u8]) -> bool {
    let message = deep_hash(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(BASE64URL.decode(tx_id.as_bytes()).unwrap().into()),
        DeepHashChunk::Chunk(BASE64URL.decode(bundler_address.as_bytes()).unwrap().into())
    ])).await.unwrap();


    let pk = Vec::new();


    lazy_static! {
        static ref PUBLIC: PKey<Public> = {
            let jwk = JWK {
                kty: "RSA",
                e: "AQAB",
                n: BASE64URL.encode(std::env::var("BUNDLER_PUBLIC"))
            };

            let p = serde_json::to_string(&jwk).unwrap();
            let key: JsonWebKey = p.parse().unwrap();
            
            PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap()
        };
    };

      
    
    let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &pkey).unwrap();
    verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    verifier.update(&message).unwrap();
    verifier.verify(sig).unwrap_or(false)
}

async fn sign_body(tx_id: &str, bundler_address: &str) -> Bytes {
    let message = deep_hash(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(BASE64URL.decode(tx_id.as_bytes()).unwrap().into()),
        DeepHashChunk::Chunk(BASE64URL.decode(bundler_address.as_bytes()).unwrap().into())
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

    sig.into()
}