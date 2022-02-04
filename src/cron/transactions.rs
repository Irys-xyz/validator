use serde::{Deserialize, Serialize};

use super::{bundle::Bundler, error::TxsError};

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct BundleTransaction {
  pub data_item_id : String,
  pub address: String,
  pub current_block: i64,
  pub expected_block: i64
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlNodes {
  pub node: BundleTransaction,
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
  pub transaction: GraphqlEdges,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlQueryResponse {
  pub data: TransactionsGqlResponse,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GqlVariables {
  pub limit: i64,
  pub after: Option<String>
}
#[derive(Deserialize, Serialize, Debug)]

pub struct ReqBody {
  pub query: String,
  pub variables: GqlVariables
}

pub async fn get_transactions(
  bundler: &Bundler,
  limit: Option<i64>,
  after: Option<String>
) -> Result<(Vec<BundleTransaction>, bool, Option<String>), TxsError> {
  let raw_query = format!("query($limit: Int, $after: String) {{ transaction(limit: $limit, after: $after) {{ pageInfo {{ hasNextPage }} edges {{ cursor node {{ data_item_id address current_block expected_block }} }} }} }}");

  let raw_variables = format!("{{\"limit\": {}, \"after\": {}}}",
    match limit {
      None => format!(r"10"),
      Some(a) => format!(r"{}", a)
    }, 
    match after {
      None => format!(r"null"),
      Some(a) => format!(r"{}", a)
  });

  let url = format!("{}/graphql", bundler.url);
  let client = reqwest::Client::new();
  let data = 
    format!("{{\"query\":\"{}\",\"variables\":{}}}", raw_query, raw_variables);

  let body = serde_json::from_str::<ReqBody>(&data);
  let res = client
    .post(&url)
    .json(&body.unwrap())
    .send()
    .await;
    
  if res.is_ok() {
    let res = res.unwrap().json::<GraphqlQueryResponse>().await.unwrap();
    let mut txs = Vec::<BundleTransaction>::new();
    let mut end_cursor: Option<String> = None;
    for tx in &res.data.transaction.edges {
      txs.push(tx.node.clone());
      end_cursor = Some(tx.cursor.clone());
    }
    let has_next_page = res.data.transaction.pageInfo.hasNextPage;

    return Ok((txs, has_next_page, end_cursor))
  }

  Err(TxsError::TxNotFound)
}  