use super::miscellaneous::ContractType;
use super::error::AnyError;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::iter::Iterator;
use graphql_client::{ GraphQLQuery, Response };

#[derive(Deserialize, Serialize, Clone)]
pub struct NetworkInfo {
  pub network: String,
  pub version: usize,
  pub release: usize,
  pub height: usize,
  pub current: String,
  pub blocks: usize,
  pub peers: usize,
  pub queue_length: usize,
  pub node_state_latency: usize,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct TransactionData {
  pub format: usize,
  pub id: String,
  pub last_tx: String,
  pub owner: String,
  pub tags: Vec<Tag>,
  pub target: String,
  pub quantity: String,
  pub data: String,
  pub reward: String,
  pub signature: String,
  pub data_size: String,
  pub data_root: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct BlockInfo {
  pub timestamp: u64,
  pub diff: String,
  pub indep_hash: String,
  pub height: u64,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct TransactionStatus {
  pub block_indep_hash: String,
}

impl TransactionData {
  pub fn get_tag(&self, tag: &str) -> Result<String, AnyError> {
    // Encodes the tag instead of decoding the keys.
    let encoded_tag = base64::encode_config(tag, base64::URL_SAFE_NO_PAD);
    self
      .tags
      .iter()
      .find(|t| t.name == encoded_tag)
      .map(|t| Ok(String::from_utf8(base64::decode(&t.value)?)?))
      .ok_or_else(|| AnyError::msg(format!("{} tag not found", tag)))?
  }
}

#[derive(Clone)]
pub enum ArweaveProtocol {
  HTTP,
  HTTPS,
}

#[derive(Clone)]
pub struct Arweave {
  pub host: String,
  pub port: i32,
  pub protocol: ArweaveProtocol,
  client: reqwest::Client,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TagFilter {
  name: String,
  values: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BlockFilter {
  max: usize,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InteractionVariables {
  tags: Vec<TagFilter>,
  owners: Vec<String>,
  block_filter: BlockFilter,
  first: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  after: Option<String>,
}

#[allow(clippy::upper_case_acronyms)]
type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "graphql/schema.graphql",
  query_path = "graphql/query.graphql",
  response_derives = "Debug"
)]
struct TransactionsView;

#[derive(Deserialize, Serialize, Clone)]
pub struct LoadedContract {
  pub id: String,
  pub contract_src_tx_id: String,
  pub contract_src: Vec<u8>,
  pub contract_type: ContractType,
  pub init_state: String,
  pub min_fee: Option<String>,
  pub contract_transaction: TransactionData,
}

enum State {
  Next(Option<String>, InteractionVariables),
  #[allow(dead_code)]
  End,
}

pub static MAX_REQUEST: usize = 100;


impl Arweave {
  pub fn new(port: i32, host: String, protocol: String) -> Arweave {
    Arweave {
      port,
      host,
      protocol: match &protocol[..] {
        "http" => ArweaveProtocol::HTTP,
        "https" | _ => ArweaveProtocol::HTTPS,
      },
      client: Client::new(),
    }
  }

  pub async fn get_tx(
    &self,
    transaction_id: &str,
  ) -> reqwest::Result<TransactionData> {
    let request = self
      .client
      .get(format!("{}/tx/{}", self.get_host(), transaction_id))
      .send()
      .await
      .unwrap();
    let transaction = request.json::<TransactionData>().await;
    transaction
  }

  pub async fn get_tx_data(&self, transaction_id: &str) -> Vec<u8> {
    let request = self
      .client
      .get(format!("{}/{}", self.get_host(), transaction_id))
      .send()
      .await
      .unwrap();
    request.bytes().await.unwrap().to_vec()
  }

  pub async fn get_tx_block(
    &self,
    transaction_id: &str,
  ) -> reqwest::Result<BlockInfo> {
    let request = self
      .client
      .get(format!("{}/tx/{}/status", self.get_host(), transaction_id))
      .send()
      .await?;

    let status = request.json::<TransactionStatus>().await?;
    let block_hash = status.block_indep_hash;

    let request = self
      .client
      .get(format!("{}/block/hash/{}", self.get_host(), block_hash))
      .send()
      .await?;

    request.json::<BlockInfo>().await
  }

  pub async fn get_network_info(&self) -> NetworkInfo {
    let info = self
      .client
      .get(format!("{}/info", self.get_host()))
      .send()
      .await
      .unwrap()
      .json::<NetworkInfo>()
      .await
      .unwrap();
    info
  }

  pub async fn get_latest_transactions(
    &self,
    owner: String
  ) -> Result<(), AnyError> {
    let request_body = 
      TransactionsView::build_query(transactions_view::Variables { owner });
    let url = format!("{}/graphql", self.get_host());

    let mut res = 
      self
      .client
      .post(&url)
      .json(&request_body)
      .send()
      .await?;

    let response_body: Response<transactions_view::ResponseData> = res.json().await?;

    dbg!(&response_body);

    Ok(())
  } 

  fn get_host(&self) -> String {
    let protocol = match self.protocol {
      ArweaveProtocol::HTTP => "http",
      ArweaveProtocol::HTTPS => "https",
    };

    if self.port == 80 {
      format!("{}://{}", protocol, self.host)
    } else {
      format!("{}://{}:{}", protocol, self.host, self.port)
    }
  }
}

#[cfg(test)]
mod tests {
 
}
