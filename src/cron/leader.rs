use tracing::error;

use crate::database::queries::{get_unposted_txs, update_tx};
use crate::database::models::NewTransaction;
use super::error::ValidatorCronError;

#[derive(Default)]
pub struct Validator {
    pub address: String,
    pub url: String
}

pub async fn send_txs_to_leader() -> Result<(), ValidatorCronError> {
  let res = post_transactions().await;
  Ok(())
}

pub fn get_leader() -> Result<Validator, ValidatorCronError> {
  Ok(Validator { 
    address: "address".to_string(),
    url: "url".to_string()
  })
}

pub async fn post_transactions() -> std::io::Result<()> {
  let txs = get_unposted_txs().await.unwrap();
  let leader = get_leader().unwrap();
  let client = reqwest::Client::new();

  for tx in txs {
    let req = client.post(format!("{}/{}", &leader.url, "tx"))
      .json(&tx)
      .send()
      .await;

    if req.is_ok() {
      let update = update_tx(&NewTransaction{
        id: tx.id,
        epoch: tx.epoch,
        block_promised: tx.block_promised,
        block_actual: tx.block_actual,
        signature: tx.signature,
        validated: tx.validated,
        bundle_id: tx.bundle_id,
        sent_to_leader: true
      }).await;

      if let Err(e) = update {
        error!("Error updating tx in database: {}", e);
      }
    }
  }

  Ok(())
}