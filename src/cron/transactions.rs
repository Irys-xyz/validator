use serde::{Deserialize, Serialize};

use crate::{bundler::Bundler, http::Client};

use super::error::TxsError;

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

pub async fn get_transactions<HttpClient>(
    client: &HttpClient,
    bundler: &Bundler,
    limit: Option<i64>,
    after: Option<String>,
) -> Result<(Vec<BundleTransaction>, bool, Option<String>), TxsError>
where
    HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
{
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
    let body = format!(
        "{{\"query\":\"{}\",\"variables\":{}}}",
        raw_query, raw_variables
    );

    let req = http::request::Builder::new()
        .method(http::Method::GET)
        .uri(url.to_string()) // TODO: find better way to transform Url to Uri
        .body(body)
        .unwrap(); // FIXME: do not unwrap

    let req = reqwest::Request::try_from(req).unwrap(); // FIXME: do not unwrap

    let res = client.execute(req).await.unwrap(); // FIXME: do not unwrap

    if res.status().is_success() {
        let res = res.json::<GraphqlQueryResponse>().await;
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
