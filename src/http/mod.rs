use futures::future::BoxFuture;

#[cfg(feature = "reqwest-client")]
pub mod reqwest;

pub use http::Method;

pub use http::{method, request, response};

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

#[cfg(test)]
pub mod mock {
    use std::{
        fmt,
        marker::PhantomData,
        pin::Pin,
        sync::{Arc, Mutex},
    };

    use futures::future::BoxFuture;
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
