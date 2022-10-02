use serde::{Deserialize, Serialize};

use crate::{bundlr::bundler::Bundler, http::Client};

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

    let url = bundler.url.join("/graphql").expect("Invalid URL"); // FIXME: change result to support failing here
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{bundlr::bundler::Bundler, http::reqwest::mock::MockHttpClient};
    use http::Method;
    use reqwest::{Request, Response};
    use url::Url;

    #[actix_rt::test]
    async fn get_transactions() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/graphql";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = r#"{"data":{"transaction":{"pageInfo":{"hasNextPage":true},"edges":[{"cursor":"VUpaVk02SXc4RjA1a2FTaVh5X1pCMW9KNXNlNXQ2Mk5VTkFVb01yU3l6Zw","node":{"data_item_id":"UJZVM6Iw8F05kaSiXy_ZB1oJ5se5t62NUNAUoMrSyzg","address":"2nlaQMUL6IjJve8FET5DtxdT9Fk337_bbiBmvXAWBYY","current_block":942660,"expected_block":943060}}]}}}"#;
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            });

        let bundler = Bundler::new(
            "".to_string(),
            // add slash at the end to make sure we won't accidentally duplicate those
            Url::from_str("http://example.com/").unwrap(),
        );
        let _ = super::get_transactions(&client, &bundler, None, None)
            .await
            .unwrap();
    }
}
