use crate::http::Client;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize, Serialize, Debug)]
pub struct SupportedCurrency {
    pub name: String,
    pub address: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BundlerConfig {
    pub version: String,
    pub gateway: String,
    pub addresses: Vec<SupportedCurrency>,
}

impl BundlerConfig {
    pub async fn new<HttpClient>(client: HttpClient, url: &Url) -> BundlerConfig
    where
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
    {
        let reqwest_client = reqwest::Client::new();
        let req = reqwest_client.get(url.to_string()).build().unwrap();

        let res = client.execute(req).await.expect("request failed");
        dbg!(&res);
        let data = res.text().await.unwrap();
        dbg!(&data);
        let body = serde_json::from_str::<BundlerConfig>(data.as_str());

        todo!()
    }
}

#[derive(Clone, Default)]
pub struct Bundler {
    pub address: String,
    pub url: String, // FIXME: type of this field should be Url
}

pub async fn get_bundler_config<HttpClient>(client: HttpClient, url: &Url) -> Option<BundlerConfig>
where
    HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    let reqwest_client = reqwest::Client::new();
    let req = reqwest_client.post(url.to_string()).build().unwrap();

    let res = client.execute(req).await.expect("request failed");
    let data = res.text().await.unwrap();
    dbg!(&data);
    let body = serde_json::from_str::<BundlerConfig>(data.as_str());

    todo!()
}
