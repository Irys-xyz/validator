use futures::future::BoxFuture;

#[cfg(feature = "reqwest-client")]
pub mod reqwest;

pub trait Client {
    type Request;
    type Response;
    type Error: std::fmt::Debug;

    fn execute(&self, req: Self::Request) -> BoxFuture<Result<Self::Response, Self::Error>>;
}

#[cfg(test)]
pub mod mock {
    use std::{
        fmt,
        sync::{Arc, Mutex},
    };

    use futures::future::BoxFuture;

    use super::Client;

    struct Handler<Request, Response> {
        matcher: fn(&Request) -> bool,
        response_builder: fn(&Request) -> Response,
    }

    pub struct Call<Request> {
        req: Request,
        count: usize,
    }

    pub struct When<Request, Response> {
        client: MockClient<Request, Response>,
        matcher: fn(&Request) -> bool,
    }

    impl<Request, Response> When<Request, Response> {
        fn new(client: MockClient<Request, Response>, matcher: fn(&Request) -> bool) -> Self {
            Self { client, matcher }
        }

        pub fn then(
            self,
            response_builder: fn(&Request) -> Response,
        ) -> MockClient<Request, Response> {
            self.client.register_handler(self.matcher, response_builder)
        }
    }

    #[derive(Debug)]
    pub enum MockHttpClientError {
        ResponseNotSet,
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

    pub struct MockClient<Request, Response> {
        state: Arc<Mutex<State<Request, Response>>>,
        req_eq: fn(&Request, &Request) -> bool,
    }

    impl<Request, Response> Clone for MockClient<Request, Response> {
        fn clone(&self) -> Self {
            Self {
                state: self.state.clone(),
                req_eq: self.req_eq.clone(),
            }
        }
    }

    impl<Request, Response> MockClient<Request, Response> {
        pub fn new(req_eq: fn(&Request, &Request) -> bool) -> Self {
            Self {
                state: Arc::new(Mutex::new(State::new())),
                req_eq,
            }
        }

        fn register_handler(
            self,
            matcher: fn(&Request) -> bool,
            response_builder: fn(&Request) -> Response,
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

        pub fn when(self, matcher: fn(&Request) -> bool) -> When<Request, Response> {
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

    impl<Request, Response> Client for MockClient<Request, Response>
    where
        Request: fmt::Debug,
        Response: Send + 'static,
    {
        type Request = Request;
        type Response = Response;
        type Error = MockHttpClientError;

        fn execute(&self, req: Self::Request) -> BoxFuture<Result<Self::Response, Self::Error>> {
            let mut state = self.state.lock().unwrap();
            let handler = state
                .handlers
                .iter()
                .find(|handler| (handler.matcher)(&req));
            match handler {
                Some(handler) => {
                    eprintln!("found handler");
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
                    eprintln!("no handler found for {:?}", req);
                    Box::pin(std::future::ready(Err(MockHttpClientError::ResponseNotSet)))
                }
            }
        }
    }
}
