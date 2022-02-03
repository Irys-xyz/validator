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
  pub transactions: GraphqlEdges,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlQueryResponse {
  pub data: TransactionsGqlResponse,
}

pub async fn get_transactions(
  bundler: &Bundler,
  first: Option<i64>,
  after: Option<String>
) -> Result<(Vec<BundleTransaction>, bool, Option<String>), TxsError> {
  let raw_query = format!("
    query($limit: Int, $after: String) {{
      transaction(limit: $limit, after: $after) {{
        pageInfo {{
          hasNextPage
        }}
        edges {{
          cursor
          node {{
            data_item_id
            address
            current_block
            expected_block
          }}
        }}
      }}
    }}"
  );
  let raw_variables = format!("{{
    limit: {},
    after: {}
  }}",
  first.unwrap_or(100), 
  match after {
    None => String::new(),
    Some(a) => format!(r" after: {}", a)
  });

  let url = format!("{}/graphql", bundler.url);
  let client = reqwest::Client::new();

  let res = client
    .post(&url)
    .body(format!("{{ query:{}, variables:{} }}", raw_query, raw_variables))
    .send()
    .await;
    
  dbg!(&url);
  dbg!(&res);
  if res.is_ok() {
    let res = res.unwrap().json::<GraphqlQueryResponse>().await.unwrap();
    let mut txs = Vec::<BundleTransaction>::new();
    let mut end_cursor: Option<String> = None;
    for tx in &res.data.transactions.edges {
      txs.push(tx.node.clone());
      end_cursor = Some(tx.cursor.clone());
    }
    let has_next_page = res.data.transactions.pageInfo.hasNextPage;

    return Ok((txs, has_next_page, end_cursor))
  }

  Err(TxsError::TxNotFound)
}  