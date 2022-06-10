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
    pub async fn fetch_config<HttpClient>(client: HttpClient, url: &Url) -> BundlerConfig
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
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::http::reqwest::mock::MockHttpClient;
    use http::Method;
    use reqwest::{Request, Response};

    use super::BundlerConfig;

    #[actix_rt::test]
    async fn fetch_config_should_return_ok() {
        let url = url::Url::from_str("https://example.com/").unwrap();
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
        .when(|req: &Request| {
            let url = "https://example.com/";
            req.method() == Method::GET && &req.url().to_string() == url
        })
        .then(|_: &Request| {
            let data = "{ \"version\":\"0.2.0\", \"addresses\":{ \"arweave\":\"arweave\" }, \"gateway\":\"example.com\" }";
            let response = http::response::Builder::new()
                .status(200)
                .body(data)
                .unwrap();
            Response::from(response)
        });

        BundlerConfig::fetch_config(client, &url).await;
    }
}
