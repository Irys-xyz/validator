use serde::{Deserialize, Serialize};

use super::{bundle::Bundler, error::TxsError};

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct BundleTransaction {
    pub data_item_id: String,
    pub address: String,
    pub current_block: i64,
    pub expected_block: i64,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlNodes {
    pub node: BundleTransaction,
    pub cursor: String,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlEdges {
    pub edges: Vec<GraphqlNodes>,
    pub page_info: PageInfo,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub has_next_page: bool,
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
    pub after: Option<String>,
}
#[derive(Deserialize, Serialize, Debug)]

pub struct ReqBody {
    pub query: String,
    pub variables: GqlVariables,
}

pub async fn get_transactions(
    bundler: &Bundler,
    limit: Option<i64>,
    after: Option<String>,
) -> Result<(Vec<BundleTransaction>, bool, Option<String>), TxsError> {
    let raw_query = "query($limit: Int, $after: String) { transaction(limit: $limit, after: $after) { pageInfo { hasNextPage } edges { cursor node { data_item_id address current_block expected_block } } } }".to_string();

    let raw_variables = format!(
        "{{\"limit\": {}, \"after\": {}}}",
        limit.unwrap_or(10),
        match after {
            None => r"null".to_string(),
            Some(a) => a,
        }
    );

    let url = format!("{}/graphql", bundler.url);
    let client = reqwest::Client::new();
    let data = format!(
        "{{\"query\":\"{}\",\"variables\":{}}}",
        raw_query, raw_variables
    );

    let body = serde_json::from_str::<ReqBody>(&data);
    let res = client.post(&url).json(&body.unwrap()).send().await;

    if res.is_ok() {
        let res = res.unwrap().json::<GraphqlQueryResponse>().await;
        if res.is_ok() {
            let res = res.unwrap();
            let mut txs = Vec::<BundleTransaction>::new();
            let mut end_cursor: Option<String> = None;
            for tx in &res.data.transaction.edges {
                txs.push(tx.node.clone());
                end_cursor = Some(tx.cursor.clone());
            }
            let has_next_page = res.data.transaction.page_info.has_next_page;
            return Ok((txs, has_next_page, end_cursor));
        } else {
            return Err(TxsError::TxNotFound);
        }
    }

    Err(TxsError::TxNotFound)
}
