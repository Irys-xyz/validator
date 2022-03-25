pub mod error;

use error::ArweaveError;
use paris::error;
use paris::info;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::http::{ Client, reqwest::ReqwestClient };

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
    winston: String,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct TransactionData {
    size: String,
    r#type: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct BlockInfo {
    pub id: String,
    pub timestamp: i64,
    pub height: u128,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Transaction {
    pub id: String,
    pub owner: Owner,
    pub signature: String,
    pub recipient: Option<String>,
    pub tags: Vec<Tag>,
    pub block: Option<BlockInfo>,
}

#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct GraphqlNodes {
    pub node: Transaction,
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

#[derive(Clone)]
pub enum ArweaveProtocol {
    Http,
    Https,
}

#[derive(Clone)]
pub struct Arweave {
    pub host: String,
    pub port: u16,
    pub protocol: ArweaveProtocol,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GqlVariables {
    pub owners: Vec<String>,
    pub first: u128,
    pub after: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ReqBody {
    pub query: String,
    pub variables: GqlVariables,
}

pub trait ArweaveContext<HttpClient> where HttpClient: Client {
    fn get_client(&self) -> HttpClient;
}

#[warn(dead_code)]
impl Arweave {
    pub fn new(port: u16, host: String, protocol: String) -> Arweave {
        Arweave {
            port,
            host,
            protocol: match &protocol[..] {
                "http" => ArweaveProtocol::Http,
                "https" | _ => ArweaveProtocol::Https,
            },
        }
    }

    pub async fn get_tx_data<Context, HttpClient>(
        &self,
        ctx: &Context,
        transaction_id: &str,
    ) -> reqwest::Result<String>
    where
        Context: ArweaveContext<HttpClient>,
        HttpClient: Client
    {
        info!("Downloading bundle {} content", &transaction_id);
        let raw_path = format!("./bundles/{}", transaction_id);
        let file_path = Path::new(&raw_path);
        let mut buffer = File::create(&file_path).unwrap();

        let host = reqwest::Url::parse(format!("{}/{}", self.get_host(), transaction_id).as_str()).unwrap();
        let req : <reqwest::Client as Client>::Request = <reqwest::Client as Client>::Request::new(http::Method::GET, host);

        let response = ctx.get_client().execute(req).await;
        if response.is_ok() {
            let mut stream = reqwest::get(host).await?.bytes_stream();
 
            while let Some(item) = stream.next().await {
                if let Err(r) = item {
                    error!("Error writing on file {:?}: {:?}", file_path.to_str(), r);
                    return Err(r);
                } else {
                    match buffer.write(&item.unwrap()) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("Error writing on file {:?}: {:?}", file_path.to_str(), err)
                        }
                    }
                }
            }

            return Ok(String::from(file_path.to_string_lossy()));
        }

        Err(response.error_for_status().err().unwrap())
    }

    pub async fn get_latest_transactions<Context, HttpClient>(
        &self,
        ctx: &Context,
        owner: &str,
        first: Option<i64>,
        after: Option<String>,
    ) -> Result<(Vec<Transaction>, bool, Option<String>), ArweaveError>
    where
        Context: ArweaveContext<HttpClient>,
        HttpClient: Client
    {
        let raw_query = "query($owners: [String!], $first: Int) { transactions(owners: $owners, first: $first) { pageInfo { hasNextPage } edges { cursor node { id owner { address } signature recipient tags { name value } block { height id timestamp } } } } }";
        let raw_variables = format!(
            "{{\"owners\": [\"{}\"], \"first\": {}, \"after\": {}}}",
            owner,
            first.unwrap_or(10),
            match after {
                None => r"null".to_string(),
                Some(a) => a,
            }
        );

        let url = format!("{}/graphql?query={}", self.get_host(), raw_query);
        let client = ctx.get_client();
        let data = format!(
            "{{\"query\":\"{}\",\"variables\":{}}}",
            raw_query, raw_variables
        );

        let body = serde_json::from_str::<ReqBody>(&data);
        /*let req = request::Builder::new()
            .method(Method::POST)
            .uri(http::uri::Uri::from_str(&url).unwrap())
            .body(&body.unwrap())
            .unwrap();
        let res = client.post(&url).json(&body.unwrap()).send().await;
        let status = res.as_ref().unwrap().status().as_u16();
 
        match status {
            200 => {
                let res = res.unwrap().json::<GraphqlQueryResponse>().await.unwrap();
                let mut txs: Vec<Transaction> = Vec::<Transaction>::new();
                let mut end_cursor: Option<String> = None;
                for tx in &res.data.transactions.edges {
                    txs.push(tx.node.clone());
                    end_cursor = Some(tx.cursor.clone());
                }
                let has_next_page = res.data.transactions.page_info.has_next_page;

                Ok((txs, has_next_page, end_cursor))
            }
            400 => Err(ArweaveError::MalformedQuery),
            404 => Err(ArweaveError::TxsNotFound),
            500 => Err(ArweaveError::InternalServerError),
            504 => Err(ArweaveError::GatewayTimeout),
            _ => Err(ArweaveError::UnknownErr),
        }
        */
        let mut txs: Vec<Transaction> = Vec::<Transaction>::new();
        Ok((txs, false, Some("end_cursor".to_owned())))
    }

    fn get_host(&self) -> String {
        let protocol = match self.protocol {
            ArweaveProtocol::Http => "http",
            ArweaveProtocol::Https => "https",
        };

        if self.port == 80 {
            format!("{}://{}", protocol, self.host)
        } else {
            format!("{}://{}:{}", protocol, self.host, self.port)
        }
    }
}

#[cfg(test)]
mod tests {}
