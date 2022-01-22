
use std::path::Path;

use awc::Client;
use bundlr_sdk::verify::{file::verify_file_bundle};
use paris::error;
use serde::{Deserialize, Serialize};
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
            let tx = Tx {
                id: transaction.id.clone(),
                block_height: match &transaction.block {
                    Some(b) => Some(b.height),
                    None => None
                }
            };

            let file_path = arweave.get_tx_data(&tx.id).await.unwrap();
            let bundle_txs = match verify_file_bundle(file_path).await {
                Err(r) => {
                    dbg!(r);
                    Vec::new()
                },
                Ok(v) => v,
            };
            
            for bundle_tx in bundle_txs {
                let tx_receipt = if let Ok(tx_receipt) = tx_exists_in_db(tx.id.as_str()).await {
                    tx_receipt
                } else if let Ok(tx_receipt) = tx_exists_on_peers(tx.id.as_str()).await {
                    tx_receipt
                } else {
                    continue;
                };

                println!("Tx receipt: {:?}", &tx_receipt);
                // Verify tx receipt

            }
        }
    }

    // If no - sad

    // If yes - check that block_seeded == block_expected

    // If valid - return

    // If not - vote to slash... once vote is confirmed then tell all peers to check

    Ok(())
}

async fn tx_exists_in_db(tx_id: &str) -> Result<TxReceipt, ValidatorCronError> {
    Ok(TxReceipt::default())
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