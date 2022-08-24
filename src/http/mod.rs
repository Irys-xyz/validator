use std::str::FromStr;

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::Future;

#[cfg(feature = "reqwest-client")]
pub mod reqwest;

pub use http::Method;

pub use http::{method, request, response};

use crate::retry;

pub trait ClientAccess<HttpClient>
where
    HttpClient: Client,
{
    fn get_http_client(&self) -> &HttpClient;
}

pub trait Client {
    type Request;
    type Response;
    type Error: std::fmt::Debug;

    fn execute(&self, req: Self::Request) -> BoxFuture<Result<Self::Response, Self::Error>>;
}

#[derive(Debug, PartialEq)]
pub enum RetryAfter {
    Timestamp(DateTime<Utc>),
    Duration(i64),
}

impl FromStr for RetryAfter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(after_seconds) = s.parse::<i64>() {
            Ok(RetryAfter::Duration(after_seconds))
        } else if let Ok(after_datetime) = httpdate::parse_http_date(s) {
            Ok(RetryAfter::Timestamp(DateTime::from(after_datetime)))
        } else {
            Err(String::from("Not a valid value for Retry-After header"))
        }
    }
}

#[cfg(test)]
pub mod mock {
    use std::{
        fmt,
        marker::PhantomData,
        pin::Pin,
        sync::{Arc, Mutex},
    };

    use futures::{future::BoxFuture, Future};
    use log::error;

    use super::Client;

    struct Handler<Request, Response> {
        matcher: fn(&Request) -> bool,
        response_builder: Pin<Box<dyn Fn(&Request) -> Response>>,
    }

    unsafe impl<Request, Response> Send for Handler<Request, Response>
    where
        Request: Send,
        Response: Send,
    {
    }

    pub struct Call<Request> {
        req: Request,
        count: usize,
    }

    impl<Request> Call<Request> {
        pub fn count(&self) -> usize {
            self.count
        }
    }

    pub struct When<Request, Response, Error> {
        client: MockClient<Request, Response, Error>,
        matcher: fn(&Request) -> bool,
    }

    impl<Request, Response, Error> When<Request, Response, Error> {
        fn new(
            client: MockClient<Request, Response, Error>,
            matcher: fn(&Request) -> bool,
        ) -> Self {
            Self { client, matcher }
        }

        pub fn then<F>(self, response_builder: F) -> MockClient<Request, Response, Error>
        where
            F: Fn(&Request) -> Response + 'static,
        {
            self.client
                .register_handler(self.matcher, Box::pin(response_builder))
        }
    }

    #[derive(Debug)]
    pub enum MockHttpClientError<ImplError> {
        ResponseNotSet,
        ImplError(ImplError),
    }

    struct State<Request, Response> {
        handlers: Vec<Handler<Request, Response>>,
        calls: Vec<Call<Request>>,
    }

    impl<Request, Response> State<Request, Response> {
        fn new() -> Self {
            Self {
                handlers: Vec::new(),
                calls: Vec::new(),
            }
        }
    }

    pub struct MockClient<Request, Response, Error> {
        state: Arc<Mutex<State<Request, Response>>>,
        req_eq: fn(&Request, &Request) -> bool,
        phantom: PhantomData<Error>,
    }

    impl<Request, Response, Error> Clone for MockClient<Request, Response, Error> {
        fn clone(&self) -> Self {
            Self {
                state: self.state.clone(),
                req_eq: self.req_eq.clone(),
                phantom: PhantomData,
            }
        }
    }

    impl<Request, Response, Error> MockClient<Request, Response, Error> {
        pub fn new(req_eq: fn(&Request, &Request) -> bool) -> Self {
            Self {
                state: Arc::new(Mutex::new(State::new())),
                req_eq,
                phantom: PhantomData,
            }
        }

        fn register_handler(
            self,
            matcher: fn(&Request) -> bool,
            response_builder: Pin<Box<dyn Fn(&Request) -> Response>>,
        ) -> Self {
            {
                let mut state = self.state.lock().unwrap();
                state.handlers.push(Handler {
                    matcher,
                    response_builder,
                });
            }
            self
        }

        pub fn when(self, matcher: fn(&Request) -> bool) -> When<Request, Response, Error> {
            When::new(self, matcher)
        }

        pub fn verify(self, verifier: fn(Vec<Call<Request>>)) {
            if let Ok(state) = Arc::try_unwrap(self.state) {
                let calls = state.into_inner().unwrap().calls;
                (verifier)(calls)
            } else {
                panic!("Cannot call verify while the mock client is still in use")
            }
        }
    }

    impl<Request, Response, Error> Client for MockClient<Request, Response, Error>
    where
        Request: fmt::Debug,
        Response: Send + 'static,
        Error: fmt::Debug + Send,
    {
        type Request = Request;
        type Response = Response;
        type Error = MockHttpClientError<Error>;

        fn execute(&self, req: Self::Request) -> BoxFuture<Result<Self::Response, Self::Error>> {
            let mut state = self.state.lock().unwrap();
            let handler = state
                .handlers
                .iter()
                .find(|handler| (handler.matcher)(&req));
            match handler {
                Some(handler) => {
                    let res = (handler.response_builder)(&req);
                    if let Some(call) = state
                        .calls
                        .iter_mut()
                        .find(|call| (self.req_eq)(&call.req, &req))
                    {
                        call.count += 1;
                    } else {
                        state.calls.push(Call { req, count: 1 })
                    }
                    Box::pin(std::future::ready(Ok(res)))
                }
                None => {
                    error!("no handler found for {:?}", req);
                    Box::pin(std::future::ready(Err(MockHttpClientError::ResponseNotSet)))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use http::HeaderValue;

    use super::RetryAfter;

    #[test]
    fn parse_duration_in_seconds() {
        let val = HeaderValue::from_static("1");
        let retry_after: RetryAfter = val.to_str().unwrap().parse().unwrap();

        assert_eq!(retry_after, RetryAfter::Duration(1))
    }

    #[test]
    fn parse_imf_fixdate_date() {
        let val = HeaderValue::from_static("Sun, 06 Nov 1994 08:49:37 GMT");
        let retry_after: RetryAfter = val.to_str().unwrap().parse().unwrap();

        assert_eq!(
            retry_after,
            RetryAfter::Timestamp(DateTime::<Utc>::from_utc(
                NaiveDate::from_ymd(1994, 11, 6).and_hms(8, 49, 37),
                Utc
            ))
        )
    }

    #[test]
    fn parse_rfc850_date() {
        let val = HeaderValue::from_static("Sunday, 06-Nov-94 08:49:37 GMT");
        let retry_after: RetryAfter = val.to_str().unwrap().parse().unwrap();

        assert_eq!(
            retry_after,
            RetryAfter::Timestamp(DateTime::<Utc>::from_utc(
                NaiveDate::from_ymd(1994, 11, 6).and_hms(8, 49, 37),
                Utc
            ))
        )
    }

    #[test]
    fn parse_asctime_date() {
        let val = HeaderValue::from_static("Sun Nov  6 08:49:37 1994");
        let retry_after: RetryAfter = val.to_str().unwrap().parse().unwrap();

        assert_eq!(
            retry_after,
            RetryAfter::Timestamp(DateTime::<Utc>::from_utc(
                NaiveDate::from_ymd(1994, 11, 6).and_hms(8, 49, 37),
                Utc
            ))
        )
    }
}
