use super::gql_result::GQLNodeParent;
use super::gql_result::GQLResultInterface;
use super::gql_result::GQLTransactionsResultInterface;
use super::gql_result::{GQLBundled, GQLEdgeInterface};
use super::miscellaneous::get_contract_type;
use super::miscellaneous::ContractType;
use super::utils::decode_base_64;
use super::error::AnyError;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::iter::Iterator;
use futures::{ stream, StreamExt };

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

#[derive(Deserialize, Serialize)]
pub struct GraphqlQuery {
  query: String,
  variables: InteractionVariables,
}

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

  pub async fn get_interactions(
    &self,
    contract_id: String,
    height: Option<usize>,
  ) -> Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError> {
    let interactions: Option<Vec<GQLEdgeInterface>> = None;

    let height_result = match height {
      Some(size) => size,
      None => self.get_network_info().await.height,
    };

    let variables = self
      .get_default_gql_variables(
        vec![], 
        Some(contract_id.to_owned()),
        None,
        height_result)
      .await;

    let mut final_result: Vec<GQLEdgeInterface> = Vec::new();

    let transactions = self
      .get_next_interaction_page(variables.clone(), false, None)
      .await?;

    let mut tx_infos = transactions.edges.clone();

    let mut cursor: Option<String> = None;
    let max_edge = self.get_max_edges(&transactions.edges);
    let maybe_edge = transactions.edges.get(max_edge);

    if let Some(data) = maybe_edge {
      let owned = data;
      cursor = Some(owned.cursor.to_owned());
    }

    let results = self.stream_interactions(cursor, variables).await;

    for result in results {
      let mut new_tx_infos = result.edges.clone();
      tx_infos.append(&mut new_tx_infos);
    }

    final_result.append(&mut tx_infos);

    let to_return: Vec<GQLEdgeInterface>;

    let filtered: Vec<GQLEdgeInterface> = final_result
      .into_iter()
      .filter(|p| {
        (p.node.parent.is_none())
          || p
            .node
            .parent
            .as_ref()
            .unwrap_or(&GQLNodeParent { id: None })
            .id
            .is_none()
          || (p.node.bundledIn.is_none())
          || p
            .node
            .bundledIn
            .as_ref()
            .unwrap_or(&GQLBundled { id: None })
            .id
            .is_none()
      })
      .collect();

    to_return = filtered;

    let are_there_new_interactions = false;
    Ok((
      to_return,
      0,
      are_there_new_interactions,
    ))
  }

  pub async fn get_latest_transactions(
    &self,
    address: String,
    app_name: String,
    height: Option<usize>,
  ) -> Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError> {

    let height_result = match height {
      Some(size) => size,
      None => self.get_network_info().await.height,
    };

    let variables = self
      .get_default_gql_variables(
        vec![address.to_owned()],
        None,
        Some(app_name.to_owned()),
        height_result)
      .await;

    let mut final_result: Vec<GQLEdgeInterface> = Vec::new();

    let transactions = self
      .get_next_interaction_page(variables.clone(), false, None)
      .await?;

    let mut tx_infos = transactions.edges.clone();

    let mut cursor: Option<String> = None;
    let max_edge = self.get_max_edges(&transactions.edges);
    let maybe_edge = transactions.edges.get(max_edge);

    if let Some(data) = maybe_edge {
      let owned = data;
      cursor = Some(owned.cursor.to_owned());
    }

    let results = self.stream_interactions(cursor, variables).await;

    for result in results {
      let mut new_tx_infos = result.edges.clone();
      tx_infos.append(&mut new_tx_infos);
    }

    final_result.append(&mut tx_infos);

    let to_return: Vec<GQLEdgeInterface>;

    let filtered: Vec<GQLEdgeInterface> = final_result
      .into_iter()
      .filter(|p| {
        (p.node.parent.is_none())
          || p
            .node
            .parent
            .as_ref()
            .unwrap_or(&GQLNodeParent { id: None })
            .id
            .is_none()
          || (p.node.bundledIn.is_none())
          || p
            .node
            .bundledIn
            .as_ref()
            .unwrap_or(&GQLBundled { id: None })
            .id
            .is_none()
      })
      .collect();

    to_return = filtered;

    let are_there_new_interactions = false;
    Ok((
      to_return,
      0,
      are_there_new_interactions,
    ))
  } 

  async fn get_next_interaction_page(
    &self,
    mut variables: InteractionVariables,
    from_last_page: bool,
    max_results: Option<usize>,
  ) -> Result<GQLTransactionsResultInterface, AnyError> {
    let mut query = String::from(
      r#"query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
        transactions(tags: $tags, block: $blockFilter, first: $first, sort: HEIGHT_ASC, after: $after) {
          pageInfo {
            hasNextPage
          }
          edges {
            node {
              id
              owner { address }
              recipient
              tags {
                name
                value
              }
              block {
                height
                id
                timestamp
              }
              fee { winston }
              quantity { winston }
              parent { id }
            }
            cursor
          }
        }
      }"#,
    );

    if from_last_page {
      query = query.replace("HEIGHT_ASC", "HEIGHT_DESC");
      variables.first = max_results.unwrap_or(100);
    }

    let graphql_query = GraphqlQuery { query, variables };
    let gql_json = serde_json::to_string(&graphql_query).unwrap();

    let req_url = format!("{}/graphql", self.get_host());
    println!("{:?}", req_url);

    dbg!(self
      .client
      .post(&req_url)
      .json(&graphql_query)
      .send()
    .await);

    let result = self
      .client
      .post(req_url)
      .json(&graphql_query)
      .send()
      .await
      .unwrap();
      
    println!("{:?}", result);
    let data = result.json::<GQLResultInterface>().await?;
      
    Ok(data.data.transactions)
  }

  pub async fn load_contract(
    &self,
    contract_id: String,
    contract_src_tx_id: Option<String>,
    contract_type: Option<String>,
  ) -> Result<LoadedContract, AnyError> {
    let mut result: Option<LoadedContract> = None;

    if result.is_some() {
      Ok(result.unwrap())
    } else {
      let contract_transaction = self.get_tx(&contract_id).await?;

      let contract_src = contract_src_tx_id
        .or_else(|| contract_transaction.get_tag("Contract-Src").ok())
        .ok_or_else(|| {
          AnyError::msg("Contract-Src tag not found in transaction")
        })?;

      let min_fee = contract_transaction.get_tag("Min-Fee").ok();

      let contract_src_tx = self.get_tx(&contract_src).await?;

      let contract_src_data =
        self.get_tx_data(&contract_src_tx.id).await;

      let mut state: String;

      if let Ok(init_state_tag) = contract_transaction.get_tag("Init-State") {
        state = init_state_tag;
      } else if let Ok(init_state_tag_txid) =
        contract_transaction.get_tag("Init-State-TX")
      {
        let init_state_tx = self.get_tx(&init_state_tag_txid).await?;
        state = decode_base_64(init_state_tx.data);
      } else {
        state = decode_base_64(contract_transaction.data.to_owned());

        if state.is_empty() {
          state = String::from_utf8(
            self.get_tx_data(&contract_transaction.id).await,
          )
          .unwrap();
        }
      }

      let contract_type = get_contract_type(
        contract_type,
        &contract_transaction,
        &contract_src_tx,
      )?;

      let final_result = LoadedContract {
        id: contract_id,
        contract_src_tx_id: contract_src,
        contract_src: contract_src_data,
        contract_type,
        init_state: state,
        min_fee,
        contract_transaction,
      };

      Ok(final_result)
    }
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

  async fn get_default_gql_variables(
    &self,
    owners: Vec::<String>,
    contract_id: Option<String>,
    app_name: Option<String>,
    height: usize,
  ) -> InteractionVariables {
    let app_name_tag: TagFilter = match app_name {
      Some(name) => TagFilter {
        name: name.to_owned(),
        values: vec!["binary".to_owned()],
      },
      None => TagFilter {
        name: "App-Name".to_owned(),
        values: vec!["SmartWeaveAction".to_owned()],
      }
    };

    let variables: InteractionVariables = match contract_id {
      Some(id) => InteractionVariables {
        tags: vec![app_name_tag, TagFilter {
          name: "Contract".to_owned(),
          values: vec![id],
        }],
        owners: owners,
        block_filter: BlockFilter { max: height },
        first: MAX_REQUEST,
        after: None,
      },
      None => InteractionVariables {
        tags: vec![app_name_tag],
        owners: owners,
        block_filter: BlockFilter { max: height },
        first: MAX_REQUEST,
        after: None,
      }
    }; 
    variables
  }

  async fn stream_interactions(
    &self,
    cursor: Option<String>,
    variables: InteractionVariables,
  ) -> Vec<GQLTransactionsResultInterface> {
    stream::unfold(State::Next(cursor, variables), |state| async move {
      match state {
        State::End => None,
        State::Next(cursor, variables) => {
          let mut new_variables: InteractionVariables = variables.clone();

          new_variables.after = cursor;

          let tx = self
            .get_next_interaction_page(new_variables, false, None)
            .await
            .unwrap();

          if tx.edges.is_empty() {
            None
          } else {
            let max_requests = self.get_max_edges(&tx.edges);

            let edge = tx.edges.get(max_requests);

            if let Some(result_edge) = edge {
              let cursor = (&result_edge.cursor).to_owned();
              Some((tx, State::Next(Some(cursor), variables)))
            } else {
              None
            }
          }
        }
      }
    })
    .collect::<Vec<GQLTransactionsResultInterface>>()
    .await
  }

  fn get_max_edges(&self, data: &[GQLEdgeInterface]) -> usize {
    let len = data.len();
    if len == MAX_REQUEST {
      MAX_REQUEST - 1
    } else if len == 0 {
      len
    } else {
      len - 1
    }
  }

  async fn has_more(
    &self,
    variables: &InteractionVariables,
    cursor: String,
  ) -> Result<bool, AnyError> {
    let mut variables = variables.to_owned();
    variables.after = Some(cursor);
    variables.first = 1;

    let load_transactions = self
      .get_next_interaction_page(variables, false, None)
      .await?;

    Ok(!load_transactions.edges.is_empty())
  }
}

#[cfg(test)]
mod tests {
  use super::Arweave;

  #[tokio::test]
  pub async fn test_build_host() {
    let arweave =
      Arweave::new(80, String::from("arweave.net"), String::from("http"));
    assert_eq!(arweave.get_host(), "http://arweave.net");
    let arweave =
      Arweave::new(443, String::from("arweave.net"), String::from("https"));
    assert_eq!(arweave.get_host(), "https://arweave.net:443");
    let arweave =
      Arweave::new(500, String::from("arweave.net"), String::from("adksad"));
    assert_eq!(arweave.get_host(), "https://arweave.net:500");
  }
}
