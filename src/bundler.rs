use crate::http::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Deserialize, Serialize, Debug)]
pub struct BundlerConfig {
    pub version: String,
    pub gateway: String,
    pub addresses: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct Bundler {
    pub address: String,
    pub url: String, // FIXME: type of this field should be Url
}

impl BundlerConfig {
    pub async fn new<HttpClient>(client: HttpClient, url: &Url) -> BundlerConfig
    where
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
    {
        let reqwest_client = reqwest::Client::new();
        let req = reqwest_client.get(url.to_string()).build().unwrap();

        let res = client.execute(req).await.expect("request failed");
        let data = res.text().await.unwrap();
        let body = serde_json::from_str::<BundlerConfig>(data.as_str());

        body.unwrap()
    }
}
