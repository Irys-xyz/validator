
use awc::Client;
use bundlr_sdk::verify::{file::verify_file_bundle};
use paris::error;
use serde::{Deserialize, Serialize};
use super::arweave::gql_result::{GQLEdgeInterface};
use super::arweave::error::AnyError;
use crate::types::Validator;
use crate::cron::arweave::arweave::Arweave;
use super::error::ValidatorCronError;

#[derive(Default)]
pub struct Bundler {
    address: String,
    url: String
}

#[derive(Serialize, Deserialize, Default)]
pub struct TxReceipt {
    block: u64,
    tx_id: String,
    signature: String
}

pub struct Tx {
    id: String,
    block: u64
}

pub async fn get_bundler() -> Result<Bundler, ValidatorCronError> {
    Ok(Bundler { 
                address: "OXcT1sVRSA5eGwt2k6Yuz8-3e3g9WJi5uSE99CWqsBs".to_string(),
                url: "url".to_string()
            })
}

pub async fn validate_bundler(bundler: Bundler) -> Result<(), ValidatorCronError> {
    // Get latest txs for bundler
    // TODO: Get tx info
    //let txs = get_txs(bundler, from_last_page, max_results);
    let txs = Vec::<Tx>::new();

    let arweave = Arweave::new(80, String::from("arweave.net"), String::from("http"));
    let transactions =
      arweave
      .get_latest_transactions(String::from("OXcT1sVRSA5eGwt2k6Yuz8-3e3g9WJi5uSE99CWqsBs"))
      .await;

    for tx in &txs {
      println!("{:?}", tx.id);
    }

    /*
    let response = client
        .get(format!("{}//{}", bundler.url, bundler.address))
        .send()
        .await;
    */

    // For each tx see if I or my peers have the tx in their db
    for tx in txs {
        // TODO: Check seeded
        // TODO: Download bundle

        let bundle_txs = verify_file_bundle("filename".to_string()).await.unwrap();
        for bundle_tx in bundle_txs {
            let tx_receipt = if let Ok(tx_receipt) = tx_exists_in_db(tx.id.as_str()).await {
                tx_receipt
            } else if let Ok(tx_receipt) = tx_exists_on_peers(tx.id.as_str()).await {
                tx_receipt
            } else {
                continue;
            };

            // Verify tx receipt
        }
    };

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