use super::error::ArweaveError;
use super::error::AnyError;
use reqwest::Client;
use reqwest::blocking::Response;
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

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Owner {
  address: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Fee {
  winston: u64
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct BlockData {
  size: u64,
  r#type: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct BlockInfo {
  pub id: String,
  pub timestamp: u64,
  pub height: u64,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct TransactionData {
  pub id: String,
  pub owner: Owner,
  pub signature: String,
  pub recipient: String,
  pub tags: Vec<Tag>,
  pub block: Option<BlockInfo>,
  pub fee: Fee,
  pub quantity: Fee,
  pub data: BlockData
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct GraphqlEdges {
  pub edges: Vec<TransactionData>
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct TransactionsGqlResponse {
  pub transactions: GraphqlEdges,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct GraphqlQueryResponse {
  pub data: TransactionsGqlResponse,
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
  ) -> Result<Response, ArweaveError> {

    let raw_query = format!("
      query {{
        transactions(owners:[\"{}\"]) {{
          edges {{
            node {{
              id
              owner {{ address }}
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
              parent {{ id }}
              data {{
                size
                type
              }}
            }}
          }}
        }}
      }}",
      owner
    );

    /*
    let query = 
    TransactionsView::build_query(
      transactions_view::Variables { 
        owner: String::from("u-x-xjbD0RDR1RaBOtZAGdSq7TZynpl9UUYcvsnvnJo") 
      });
    let json_query = serde_json::to_string(&query.query).unwrap();
    let json_variables = serde_json::to_string(&query.variables).unwrap();
    */

    let url = format!("{}/graphql?query={}", self.get_host(), raw_query);
    let client = reqwest::Client::new();

    let res = 
      client
      .post(&url)
      .send()
      .await;

    /*
    if res.is_ok() {
      Ok(res.unwrap().json().await);
    }
    */
    dbg!("{}", res.unwrap().text().await);

    Err(ArweaveError::TxNotFound)
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
