use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use bundlr_sdk::deep_hash::{deep_hash, DeepHashChunk, ONE_AS_BUFFER};

use data_encoding::BASE64URL_NOPAD;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use paris::error;
use serde::{Deserialize, Serialize};

use crate::{
    consts::{BUNDLR_AS_BUFFER, VALIDATOR_AS_BUFFER},
    database::{models::NewTransaction, schema::transactions::dsl::*},
    key_manager,
    server::{error::ValidatorServerError, RuntimeContext},
    state::{ValidatorRole, ValidatorStateAccess},
};

pub trait Config<KeyManager>: ValidatorStateAccess
where
    KeyManager: key_manager::KeyManager,
{
    fn bundler_address(&self) -> &str;
    fn validator_address(&self) -> &str;
    fn key_manager(&self) -> &KeyManager;
    fn current_epoch(&self) -> i64;
    fn current_block(&self) -> u128;
}

#[derive(Deserialize)]
pub struct UnsignedBody {
    id: String,
    signature: String,
    block: u128,
    validators: Vec<String>,
}

impl UnsignedBody {
    // FIXME: needs proper error type
    pub async fn verify<KeyManager>(&self, key_manager: &KeyManager) -> Result<bool, ()>
    where
        KeyManager: key_manager::KeyManager,
    {
        let block = self.block.to_string().as_bytes().to_vec();
        let tx_id = self.id.as_bytes().to_vec();

        let signature_data = deep_hash(DeepHashChunk::Chunks(vec![
            DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
            DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
            DeepHashChunk::Chunk(tx_id.into()),
            DeepHashChunk::Chunk(block.into()),
        ]))
        .await
        .map_err(|err| {
            error!("Failed to build data for signing: {:?}", err);
        })?;

        let decoded_signature =
            BASE64URL_NOPAD
                .decode(self.signature.as_bytes())
                .map_err(|err| {
                    error!("Failed to decode signature: {:?}", err);
                })?;

        Ok(key_manager.verify_bundler_signature(&signature_data, &decoded_signature))
    }

    // FIXME: needs proper error type
    pub async fn sign<KeyManager>(&self, key_manager: &KeyManager) -> Result<String, ()>
    where
        KeyManager: key_manager::KeyManager,
    {
        let tx_id = self.id.as_bytes().to_vec();
        let bundler_address = key_manager.bundler_address().to_string();

        let signature_data = deep_hash(DeepHashChunk::Chunks(vec![
            DeepHashChunk::Chunk(VALIDATOR_AS_BUFFER.into()),
            DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
            DeepHashChunk::Chunk(tx_id.into()),
            DeepHashChunk::Chunk(bundler_address.into()),
        ]))
        .await
        .map_err(|err| {
            error!("Failed to build data for signing: {:?}", err);
        })?;

        Ok(BASE64URL_NOPAD.encode(&key_manager.validator_sign(&signature_data)))
    }
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

pub async fn sign_route<Context, KeyManager>(
    ctx: Data<Context>,
    body: Json<UnsignedBody>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Context: self::Config<KeyManager> + RuntimeContext + Send,
    KeyManager: key_manager::KeyManager,
{
    if ctx.get_validator_state().role() != ValidatorRole::Cosigner {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let body = body.into_inner();

    // Verify
    let exists = {
        let conn = ctx.get_db_connection();
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

    let current_block = ctx.current_block();
    let key_manager = ctx.key_manager();

    if !body
        .validators
        .contains(&ctx.validator_address().to_string())
    {
        return Ok(HttpResponse::BadRequest().finish());
    }

    if body.block < (current_block - 5) || body.block > (current_block + 5) {
        return Ok(HttpResponse::BadRequest().finish());
    }

    match body.verify(key_manager).await {
        Ok(true) => (),
        Ok(false) => return Ok(HttpResponse::BadRequest().finish()),
        Err(()) => return Err(ValidatorServerError::InternalError),
    };

    // Sign
    let sig = match body.sign(key_manager).await {
        Ok(sig) => sig,
        Err(()) => return Err(ValidatorServerError::InternalError),
    };

    // Add to db
    let current_epoch = ctx.current_epoch();

    let new_transaction = NewTransaction {
        id: body.id,
        epoch: current_epoch,
        block_promised: i64::try_from(body.block).unwrap(), // FIXME: don't unwrap
        block_actual: None,
        signature: sig.as_bytes().to_vec(),
        validated: false,
        bundle_id: None,
        sent_to_leader: false,
    };

    let conn = ctx.get_db_connection();
    actix_rt::task::spawn_blocking(move || {
        diesel::insert_into(transactions)
            .values::<NewTransaction>(new_transaction)
            .execute(&conn)
    })
    .await??;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .body(sig.as_bytes().to_vec()))
}

#[cfg(test)]
mod tests {
    use bundlr_sdk::{
        deep_hash::{DeepHashChunk, ONE_AS_BUFFER},
        deep_hash_sync::deep_hash_sync,
    };
    use data_encoding::BASE64URL_NOPAD;
    use openssl::{
        hash::MessageDigest,
        pkey::{PKey, Private},
        rsa::Padding,
        sign,
    };

    use crate::{
        consts::{BUNDLR_AS_BUFFER, VALIDATOR_AS_BUFFER},
        key_manager::{test_utils::test_keys, KeyManager},
    };

    use super::UnsignedBody;
    fn test_message(
        signing_key: &PKey<Private>,
        block: u128,
        validators: Vec<String>,
    ) -> UnsignedBody {
        let tx_id = "dtdOmHZMOtGb2C0zLqLBUABrONDZ5rzRh9NengT1-Zk";
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
            validators,
        }
    }

    #[actix_rt::test]
    async fn verify_unsigned_body() {
        let (key_manager, bundler_private_key) = test_keys();

        let msg = test_message(
            &bundler_private_key,
            500,
            vec![key_manager.validator_address().to_string()],
        );

        assert!(msg.verify(&key_manager).await.unwrap())
    }

    #[actix_rt::test]
    async fn sign_unsigned_body() {
        // TODO: use pre-generated keys
        // instead of using random keys, use pre-generated ones so that
        // the signature can be verified against expected value

        let (key_manager, bundler_private_key) = test_keys();

        let msg = test_message(
            &bundler_private_key,
            500,
            vec![key_manager.validator_address().to_string()],
        );

        let sig = msg.sign(&key_manager).await.unwrap();

        let bundler_address = key_manager.bundler_address().to_string();
        let signature_data = deep_hash_sync(DeepHashChunk::Chunks(vec![
            DeepHashChunk::Chunk(VALIDATOR_AS_BUFFER.into()),
            DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
            DeepHashChunk::Chunk(msg.id.into()),
            DeepHashChunk::Chunk(bundler_address.into()),
        ]))
        .unwrap();

        let decoded_signature = BASE64URL_NOPAD.decode(sig.as_bytes()).unwrap();
        assert!(key_manager.verify_validator_signature(&signature_data, &decoded_signature));
    }
}
