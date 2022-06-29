use super::arweave::TransactionData;
use super::utils::hasher;
use super::error::AnyError;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum ContractType {
  JAVASCRIPT,
  WASM,
  EVM,
}

pub fn get_contract_type(
  maybe_content_type: Option<String>,
  contract_transaction: &TransactionData,
  source_transaction: &TransactionData,
) -> Result<ContractType, AnyError> {
  let contract_type = maybe_content_type
    .or_else(|| source_transaction.get_tag("Content-Type").ok())
    .or_else(|| contract_transaction.get_tag("Content-Type").ok())
    .ok_or_else(|| {
      AnyError::msg("Contract-Src tag not found in transaction")
    })?;

  let ty = match &(contract_type.to_lowercase())[..] {
    "application/javascript" => ContractType::JAVASCRIPT,
    "application/wasm" => ContractType::WASM,
    "application/octet-stream" => ContractType::EVM,
    _ => ContractType::JAVASCRIPT,
  };

  Ok(ty)
}

pub fn get_sort_key(
  block_height: &usize,
  block_id: &str,
  transaction_id: &str,
) -> String {
  let mut hasher_bytes =
    base64::decode_config(block_id, base64::URL_SAFE_NO_PAD).unwrap();
  let mut tx_id =
    base64::decode_config(transaction_id, base64::URL_SAFE_NO_PAD).unwrap();
  hasher_bytes.append(&mut tx_id);
  let hashed = hex::encode(hasher(&hasher_bytes[..]));
  let height = format!("000000{}", *block_height);

  format!("{},{}", &height[height.len() - 12..], hashed)
}