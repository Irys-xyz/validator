extern crate diesel;

use super::arweave;
use super::error::ValidatorCronError;
use super::slasher::vote_slash;
use super::transactions::get_transactions;
use crate::context::BundlerAccess;
use crate::cron::arweave::{Arweave, Transaction as ArweaveTx};
use crate::database::models::{Block, Epoch, NewBundle, NewTransaction};
use crate::database::queries::{self, *};
use crate::http;
use crate::types::Validator;
use awc::Client;
use bundlr_sdk::deep_hash_sync::{deep_hash_sync, ONE_AS_BUFFER};
use bundlr_sdk::verify::types::Item;
use bundlr_sdk::JWK;
use bundlr_sdk::{deep_hash::DeepHashChunk, verify::file::verify_file_bundle};
use data_encoding::BASE64URL_NOPAD;
use jsonwebkey::JsonWebKey;
use lazy_static::lazy_static;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Public};
use openssl::rsa::Padding;
use openssl::sign;
use paris::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct Bundler {
    pub address: String,
    pub url: String, // FIXME: type of this field should be Url
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct TxReceipt {
    block: u128,
    tx_id: String,
    signature: String,
}

pub async fn validate_bundler<Context, HttpClient>(ctx: &Context) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext + arweave::ArweaveContext<HttpClient> + BundlerAccess,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    let arweave = Arweave::new(80, String::from("arweave.net"), String::from("http"));
    let bundler = ctx.bundler();
    let txs_req = arweave
        .get_latest_transactions(ctx, &bundler.address, Some(50), None)
        .await;

    if let Err(r) = txs_req {
        error!(
            "Error occurred while getting txs from bundler address: \n {}. Error: {}",
            bundler.address, r
        );
        return Err(ValidatorCronError::TxsFromAddressNotFound);
    }

    let txs_req = &txs_req.unwrap().0;
    for bundle_tx in txs_req {
        let res = validate_bundle(ctx, &arweave, bundle_tx).await;
        if let Err(err) = res {
            match err {
                ValidatorCronError::TxNotFound => todo!(),
                ValidatorCronError::AddressNotFound => todo!(),
                ValidatorCronError::TxsFromAddressNotFound => todo!(),
                ValidatorCronError::BundleNotInsertedInDB => todo!(),
                ValidatorCronError::TxInvalid => todo!(),
                ValidatorCronError::FileError => todo!(),
            }
        }
    }

    // If no - sad - OK

    // If yes - check that block_seeded == block_expected -

    // If valid - return

    // If not - vote to slash... once vote is confirmed then tell all peers to check

    Ok(())
}

async fn validate_bundle<Context, HttpClient>(
    ctx: &Context,
    arweave: &Arweave,
    bundle: &ArweaveTx,
) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext + arweave::ArweaveContext<HttpClient> + BundlerAccess,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    let block_ok = check_bundle_block(ctx, bundle).await;
    let current_block: Option<u128> = None;
    if let Err(err) = block_ok {
        return Err(err);
    }
    if current_block.is_none() {
        return Ok(());
    }

    let path = match arweave.get_tx_data(ctx, &bundle.id).await {
        Ok(path) => path,
        Err(err) => {
            error!("File path error {:?}", err);
            return Err(ValidatorCronError::FileError);
        }
    };

    let bundle_txs = match verify_file_bundle(path.clone()).await {
        Err(r) => {
            error!("Error verifying bundle {}:", r);
            Vec::new()
        }
        Ok(v) => v,
    };

    info!(
        "{} transactions found in bundle {}",
        &bundle_txs.len(),
        &bundle.id
    );
    for bundle_tx in bundle_txs {
        let tx_receipt = verify_bundle_tx(ctx, &bundle_tx, current_block).await;
        if let Err(err) = tx_receipt {
            info!("Error found in transaction {} : {}", &bundle_tx.tx_id, err);
            return Err(ValidatorCronError::TxInvalid);
        }
    }

    match std::fs::remove_file(path.clone()) {
        Ok(_r) => info!("Successfully deleted {}", path),
        Err(err) => error!("Error deleting file {} : {}", path, err),
    };

    Ok(())
}

async fn check_bundle_block<Context>(
    ctx: &Context,
    bundle: &ArweaveTx,
) -> Result<Option<u128>, ValidatorCronError>
where
    Context: queries::QueryContext + BundlerAccess,
{
    let current_block = match bundle.block {
        Some(ref block) => block.height,
        None => {
            info!(
                "Bundle {} not included in any block, moving on ...",
                &bundle.id
            );
            return Ok(None);
        }
    };

    info!("Bundle {} included in block {}", &bundle.id, current_block);
    let is_bundle_present = get_bundle(ctx, &bundle.id).is_ok();

    if !is_bundle_present {
        return match insert_bundle_in_db(
            ctx,
            NewBundle {
                id: bundle.id.clone(),
                owner_address: ctx.bundler().address.clone(),
                block_height: Block(current_block),
            },
        ) {
            Ok(()) => {
                info!("Bundle {} successfully stored", &bundle.id);
                Ok(Some(current_block))
            }
            Err(err) => {
                error!("Error when storing bundle {} : {}", &bundle.id, err);
                Err(ValidatorCronError::BundleNotInsertedInDB)
            }
        };
    }

    Ok(Some(current_block))
}

async fn verify_bundle_tx<Context>(
    ctx: &Context,
    bundle_tx: &Item,
    current_block: Option<u128>,
) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext,
{
    let tx = get_tx(ctx, &bundle_tx.tx_id).await;
    let mut tx_receipt: Option<TxReceipt> = None;
    if tx.is_ok() {
        let tx = tx.unwrap();
        tx_receipt = Some(TxReceipt {
            block: tx.block_promised.into(),
            tx_id: tx.id,
            signature: match std::str::from_utf8(&tx.signature.to_vec()) {
                Ok(v) => v.to_string(),
                Err(e) => panic!("Invalid UTF-8 seq: {}", e),
            },
        });
    } else {
        let peer_tx = tx_exists_on_peers(&bundle_tx.tx_id).await;
        if peer_tx.is_ok() {
            tx_receipt = Some(peer_tx.unwrap());
        }
    }

    match tx_receipt {
        Some(receipt) => {
            let tx_is_ok = verify_tx_receipt(&receipt).unwrap();
            // FIXME: don't use unwrap
            if tx_is_ok && receipt.block <= current_block.unwrap() {
                if let Err(_err) = insert_tx_in_db(
                    ctx,
                    &NewTransaction {
                        id: receipt.tx_id,
                        epoch: Epoch(0),
                        block_promised: receipt.block.into(),
                        block_actual: current_block.map(Block),
                        signature: receipt.signature.as_bytes().to_vec(),
                        validated: true,
                        bundle_id: Some(bundle_tx.tx_id.clone()),
                    },
                ) {
                    // FIXME: missing error handling
                }
            } else {
                // TODO: vote slash
            }
        }
        None => todo!(),
    }

    Ok(())
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
            return Ok(response.json().await.unwrap());
        }
    }

    Err(ValidatorCronError::TxNotFound)
}

fn verify_tx_receipt(tx_receipt: &TxReceipt) -> std::io::Result<bool> {
    pub const BUNDLR_AS_BUFFER: &[u8] = "Bundlr".as_bytes();

    let block = tx_receipt.block.to_string().as_bytes().to_vec();

    let tx_id = tx_receipt.tx_id.as_bytes().to_vec();

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
                n: std::env::var("BUNDLER_PUBLIC").unwrap(),
            };

            let p = serde_json::to_string(&jwk).unwrap();
            let key: JsonWebKey = p.parse().unwrap();

            PKey::public_key_from_der(key.key.to_der().as_slice()).unwrap()
        };
    };

    let sig = BASE64URL_NOPAD
        .decode(tx_receipt.signature.as_bytes())
        .unwrap();

    let mut verifier = sign::Verifier::new(MessageDigest::sha256(), &PUBLIC).unwrap();
    verifier.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
    verifier.update(&message).unwrap();
    Ok(verifier.verify(&sig).unwrap_or(false))
}

pub async fn validate_transactions(bundler: &Bundler) -> Result<(), ValidatorCronError> {
    let res = get_transactions(bundler, Some(100), None).await;
    let txs = match res {
        Ok(r) => r.0,
        Err(_) => Vec::new(),
    };

    for tx in txs {
        // TODO: validate transacitons
        let block_ok = tx.current_block < tx.expected_block;

        if block_ok {
            let _res = vote_slash(bundler);
        }
    }

    Ok(())
}
