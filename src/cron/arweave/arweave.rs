use super::error::{ ArweaveError, AnyError };
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::iter::Iterator;

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

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Owner {
  address: String,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Fee {
  winston: String
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct TransactionData {
  size: String,
  r#type: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct BlockInfo {
  pub id: String,
  pub timestamp: u64,
  pub height: u64,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Transaction {
  pub id: String,
  pub owner: Owner,
  pub signature: String,
  pub recipient: Option<String>,
  pub tags: Vec<Tag>,
  pub block: Option<BlockInfo>,
  pub fee: Fee,
  pub quantity: Fee,
  pub data: TransactionData
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlNodes {
  pub node: Transaction,
  pub cursor: String,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlEdges {
  pub edges: Vec<GraphqlNodes>,
  pub pageInfo: PageInfo,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct PageInfo {
  pub hasNextPage: bool
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct TransactionsGqlResponse {
  pub transactions: GraphqlEdges,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlQueryResponse {
  pub data: TransactionsGqlResponse,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct TransactionStatus {
  pub block_indep_hash: String,
}

impl Transaction {
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
  ) -> reqwest::Result<Transaction> {
    let request = self
      .client
      .get(format!("{}/tx/{}", self.get_host(), transaction_id))
      .send()
      .await
      .unwrap();
    let transaction = request.json::<Transaction>().await;
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
    owner: &String,
    first: Option<i32>,
    after: Option<String>,
  ) -> Result<(Vec<Transaction>, bool, Option<String>), ArweaveError> {
    let raw_query = format!("
      query {{
        transactions(owners:[\"{}\"] first: {} {}) {{
          pageInfo {{
            hasNextPage
          }}
          edges {{
            cursor
            node {{
              id
              owner {{ address }}
              signature
              recipient
              tags {{
                name
                value
              }}
              block {{
                height
                id
                timestamp
              }}
              fee {{ winston }}
              quantity {{ winston }}
              data {{
                size
                type
              }}
            }}
          }}
        }}
      }}",
      owner,
      first.unwrap_or(100),
      match after {
        None => format!(""),
        Some(a) => format!(r" after: {}", a)
      }
    );

    let url = format!("{}/graphql?query={}", self.get_host(), raw_query);
    let client = reqwest::Client::new();

    let res = 
      client
      .post(&url)
      .send()
      .await;

    if res.is_ok() {
      let res = res.unwrap().json::<GraphqlQueryResponse>().await.unwrap();
      let mut txs: Vec<Transaction> = Vec::<Transaction>::new();
      let mut end_cursor: Option<String> = None;
      for tx in &res.data.transactions.edges {
        txs.push(tx.node.clone());
        end_cursor = Some(tx.cursor.clone());
      }
      let has_next_page = res.data.transactions.pageInfo.hasNextPage;

      return Ok((txs, has_next_page, end_cursor))
    }

    Err(ArweaveError::TxsNotFound)
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
