use awc::Client;
use bundlr_sdk::JWK;
use bundlr_sdk::deep_hash_sync::{ deep_hash_sync, ONE_AS_BUFFER };
use bundlr_sdk::verify::types::Item;
use bundlr_sdk::{ verify::file::verify_file_bundle, deep_hash::DeepHashChunk };
use data_encoding::BASE64URL_NOPAD;
use openssl::hash::MessageDigest;
use openssl::pkey::{Public, PKey};
use openssl::rsa::Padding;
use openssl::sign;
use paris::error;
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use jsonwebkey::JsonWebKey;
use crate::types::Validator;
use crate::cron::arweave::arweave::Arweave;
use super::error::ValidatorCronError;

#[derive(Default)]
pub struct Bundler {
    address: String,
    url: String
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct TxReceipt {
    block: u64,
    tx_id: String,
    signature: String
}

pub struct Tx {
    id: String,
    block_height: Option<u64>
}

pub async fn get_bundler() -> Result<Bundler, ValidatorCronError> {
    Ok(Bundler { 
                address: "OXcT1sVRSA5eGwt2k6Yuz8-3e3g9WJi5uSE99CWqsBs".to_string(),
                url: "url".to_string()
            })
}

pub async fn validate_bundler(bundler: Bundler) -> Result<(), ValidatorCronError> {
    let arweave = Arweave::new(80, String::from("arweave.net"), String::from("http"));
    let txs_req =
      arweave
      .get_latest_transactions(&bundler.address, Some(50), None)
      .await;

    if let Err(r) = txs_req {
        error!("Error occurred while getting txs from bundler address: \n {}. \n Error: {}",
                bundler.address,
                r);
    }   else if txs_req.is_ok() {
        let txs_req = &txs_req.unwrap().0;
        for transaction in txs_req {
            // TODO: Check seeded [?]
            // TODO: Download bundle [?]
            let arweave_tx = Tx {
                id: transaction.id.clone(),
                block_height: match &transaction.block {
                    Some(b) => Some(b.height),
                    None => None
                }
            };

            let file_path = arweave.get_tx_data(&arweave_tx.id).await;
            if file_path.is_ok() {
                println!("Verifying file: {}", &file_path.as_ref().unwrap());
                let bundle_txs = match verify_file_bundle(file_path.unwrap()).await {
                    Err(r) => {
                        dbg!(r);
                        Vec::new()
                    },
                    Ok(v) => v,
                };
                
                for bundle_tx in bundle_txs {
                    let tx_receipt = if let Ok(tx_receipt) = tx_exists_in_db(&bundle_tx).await {
                        tx_receipt
                    } else if let Ok(tx_receipt) = tx_exists_on_peers(&bundle_tx.tx_id).await {
                        tx_receipt
                    } else {
                        continue;
                    };
    
                    let tx_is_ok = verify_tx_receipt(&tx_receipt);
                    println!("{:?}", tx_is_ok);
                }
            }
        }
    }

    // If no - sad

    // If yes - check that block_seeded == block_expected

    // If valid - return

    // If not - vote to slash... once vote is confirmed then tell all peers to check

    Ok(())
}

// TODO: implement the database verification correctly
async fn tx_exists_in_db(bundle_tx: &Item) -> Result<TxReceipt, ValidatorCronError> {
    Ok(TxReceipt { 
        block: 10,
        tx_id: bundle_tx.tx_id.clone(),
        signature: match String::from_utf8(bundle_tx.signature.clone()) {
            Ok(s) => s,
            Err(_) => String::new(),
        },
    })
}

async fn tx_exists_on_peers(tx_id: &str) -> Result<TxReceipt, ValidatorCronError> {
    let client = Client::default();
    let validator_peers = Vec::<Validator>::new();
    for peer in validator_peers {
        let response = client
            .get(format!("{}/tx/{}", peer.url, tx_id))
            .send()
            .await;
        
            if let Err(r) = response {
                error!("Error occurred while getting tx from peer - {}", r);
                continue;
            }

        let mut response = response.unwrap();

        if response.status().is_success() {
            return Ok(response
                            .json()
                            .await
                            .unwrap())
        }
    }

    Err(ValidatorCronError::TxNotFound)
}


fn verify_tx_receipt(tx_receipt: &TxReceipt) -> std::io::Result<bool> {
    pub const BUNDLR_AS_BUFFER: &[u8] = "Bundlr".as_bytes();

    let block = tx_receipt.block.to_string()
        .as_bytes()
        .to_vec();

    let tx_id = tx_receipt.tx_id.as_bytes().to_vec();

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
                n: std::env::var("BUNDLER_PUBLIC").unwrap()
            };

            let p = serde_json::to_string(&jwk).unwrap();
            let key: JsonWebKey = p.parse().unwrap();
            
            PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap()
        };
    };

    let sig = BASE64URL_NOPAD.decode(tx_receipt.signature.as_bytes()).unwrap();

    let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &PUBLIC).unwrap();
    verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    verifier.update(&message).unwrap();
    Ok(verifier.verify(&sig).unwrap_or(false))
}