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
    // FIXME: borrow the HttpClient instead of moving ownership
    pub async fn fetch_config<HttpClient>(client: HttpClient, url: &Url) -> BundlerConfig
    where
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
    {
        let req = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(url.to_string()) // TODO: find better way to transform Url to Uri
            .body("".to_owned()) // TODO: find better solution than creating empty string
            .expect("Failed to build request for fetching bundler config");

        let req = reqwest::Request::try_from(req)
            .expect("Failed to build request for fetching bundler config");

        let res = client
            .execute(req)
            .await
            .expect("Failed to fetch bundler config");

        let data = res
            .text()
            .await
            .expect("Failed to deserialize bundler config");

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
