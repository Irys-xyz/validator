use futures::future::BoxFuture;

#[derive(Clone)]
pub struct ReqwestClient(reqwest::Client);

impl ReqwestClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self(client)
    }
}

impl super::Client for ReqwestClient {
    type Request = reqwest::Request;
    type Response = reqwest::Response;
    type Error = reqwest::Error;

    fn execute(&self, req: Self::Request) -> BoxFuture<Result<Self::Response, Self::Error>> {
        Box::pin(self.0.execute(req))
    }
}

#[cfg(test)]
pub mod mock {
    use crate::http::mock::MockHttpClientError;

    pub type MockHttpClient =
        crate::http::mock::MockClient<reqwest::Request, reqwest::Response, reqwest::Error>;

    impl From<reqwest::Error> for MockHttpClientError<reqwest::Error> {
        fn from(err: reqwest::Error) -> Self {
            MockHttpClientError::ImplError(err)
        }
    }

    mod test {
        use std::str::FromStr;

        use http::Method;
        use reqwest::{Request, Response};
        use serde::{Deserialize, Serialize};

        use crate::http::Client;

        use super::MockHttpClient;

        #[derive(Debug, Deserialize, PartialEq, Serialize)]
        struct TestRecord {
            foo: String,
        }

        #[actix_rt::test]
        async fn mock_http_get_request() {
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

            let req: http::Request<String> = http::request::Builder::new()
                .method(http::Method::GET)
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

        #[actix_rt::test]
        async fn mock_http_post_request() {
            let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
                .when(|req: &Request| {
                    req.method() == Method::POST && &req.url().to_string() == "http://example.com/"
                })
                .then(|_: &Request| {
                    let response = http::response::Builder::new()
                        .status(201)
                        .header("Location", "/foo")
                        .body("".to_string())
                        .unwrap();
                    Response::from(response)
                });

            let record = TestRecord {
                foo: "bar".to_string(),
            };
            let req: http::Request<String> = http::request::Builder::new()
                .method(http::Method::POST)
                .uri(http::uri::Uri::from_str("http://example.com/").unwrap())
                .body(serde_json::to_string(&record).unwrap())
                .unwrap();
            let req: reqwest::Request = reqwest::Request::try_from(req).unwrap();
            let res = client.execute(req).await.unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::CREATED);
            assert_eq!(
                res.headers().get("Location").unwrap().to_str().unwrap(),
                "/foo"
            );
        }
    }
}
