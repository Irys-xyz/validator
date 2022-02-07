use crate::database::queries::get_unposted_txs;
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
  }

  Ok(())
}