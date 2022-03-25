pub type ReqwestClient = reqwest::Client;

impl super::Client for ReqwestClient {
    type Request = reqwest::Request;
    type Response = reqwest::Response;
    type Error = reqwest::Error;

    fn execute(
        &self,
        req: Self::Request,
    ) -> std::pin::Pin<Box<dyn futures::Future<Output = Result<Self::Response, Self::Error>>>> {
        Box::pin(self.execute(req))
    }
}

#[cfg(test)]
pub mod mock {
    type MockHttpClient = super::super::mock::MockClient<reqwest::Request, reqwest::Response>;

    mod test {
        use std::str::FromStr;

        use http::{request, Method};
        use reqwest::{Request, Response};
        use serde::Deserialize;

        use crate::http::Client;

        use super::MockHttpClient;

        #[derive(Debug, Deserialize, PartialEq)]
        struct TestRecord {
            foo: String,
        }

        #[actix_rt::test]
        async fn mock_http_request() {
            let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
                .when(|req: &Request| &req.url().to_string() == "http://example.com/")
                .then(|_: &Request| {
                    let response = http::response::Builder::new()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(r#"{"foo":"bar"}"#)
                        .unwrap();
                    Response::from(response)
                });

            let req: http::Request<String> = request::Builder::new()
                .method(Method::GET)
                .uri(http::uri::Uri::from_str("http://example.com/").unwrap())
                .body("".to_owned())
                .unwrap();
            let req: reqwest::Request = reqwest::Request::try_from(req).unwrap();
            let res: TestRecord = client.execute(req).await.unwrap().json().await.unwrap();
            assert_eq!(
                res,
                TestRecord {
                    foo: "bar".to_string()
                }
            );
        }
    }
}
