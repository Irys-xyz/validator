use serde::Serialize;
use tracing::error;

use super::error::ValidatorCronError;
use crate::database::models::NewTransaction;
use crate::database::queries::{get_unposted_txs, update_tx};
use crate::state::SharedValidatorState;

#[derive(Default)]
pub struct Validator {
    pub address: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct ValidatorSignature {
    public: String,
    signature: String,
}

#[derive(Serialize)]
pub struct ReqBody {
    id: String,
    signature: String,
    block: i64,
    address: String,
    validator_signatures: Vec<ValidatorSignature>,
}

pub async fn send_txs_to_leader(state: SharedValidatorState) -> Result<(), ValidatorCronError> {
    let _res = post_transactions().await;
    Ok(())
}

pub fn get_leader() -> Result<Validator, ValidatorCronError> {
    Ok(Validator {
        address: "address".to_string(),
        url: "url".to_string(),
    })
}

pub async fn post_transactions() -> std::io::Result<()> {
    let txs = get_unposted_txs().await.unwrap();
    let leader = get_leader().unwrap();
    let client = reqwest::Client::new();

    for tx in txs {
        let req = client
            .post(format!("{}/{}", &leader.url, "tx"))
            .json(&ReqBody {
                id: tx.id.clone(),
                signature: String::from_utf8(tx.signature.clone()).unwrap(),
                block: tx.block_actual.unwrap(),
                address: String::from("address"), // TODO: get this address
                validator_signatures: Vec::new(),
            })
            .send()
            .await;

        if req.is_ok() {
            let update = update_tx(&NewTransaction {
                id: tx.id,
                epoch: tx.epoch,
                block_promised: tx.block_promised,
                block_actual: tx.block_actual,
                signature: tx.signature,
                validated: tx.validated,
                bundle_id: tx.bundle_id,
                sent_to_leader: true,
            })
            .await;

            if let Err(e) = update {
                error!("Error updating tx in database: {}", e);
            }
        }
    }

    Ok(())
}
