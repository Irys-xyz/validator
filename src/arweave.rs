use data_encoding::BASE64URL_NOPAD;
use derive_more::{Display, Error};
use futures::future::BoxFuture;
use futures::{pin_mut, FutureExt, StreamExt};
use http::header::CONTENT_LENGTH;
use http::Uri;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::{self, Debug};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use std::num::ParseIntError;
use std::str::FromStr;
use url::Url;

use crate::dynamic_async_queue::DynamicAsyncQueue;
use crate::http::reqwest::execute_with_retry;
use crate::http::{Client, ClientAccess};
use crate::key_manager::public_key_to_address;

pub mod visitor {
    use futures::stream;

    use crate::context;

    use super::{ArweaveError, BlockIndepHash, BlockInfo};

    pub fn arweave_visitor<Context, HttpClient>(
        ctx: &Context,
        start: BlockIndepHash,
    ) -> impl stream::Stream<Item = Result<BlockInfo, ArweaveError>> + '_
    where
        Context: super::ArweaveContext<HttpClient> + context::ArweaveAccess + Unpin,
        HttpClient: crate::http::Client<Request = reqwest::Request, Response = reqwest::Response>
            + Clone
            + Send
            + Sync
            + 'static,
        HttpClient::Error: From<reqwest::Error> + Send,
    {
        stream::unfold((Some(start), ctx), |(next, ctx)| async move {
            match next {
                Some(block_id) => match ctx.arweave().get_block_info(ctx, &block_id).await {
                    Ok(block) => {
                        let next = block.previous_block.clone();
                        Some((Ok(block), (Some(next), ctx)))
                    }
                    err => Some((err, (None, ctx))),
                },
                None => None,
            }
        })
    }

    #[cfg(test)]
    mod tests {
        use std::{fs, str::FromStr};

        use futures::{pin_mut, stream::StreamExt};
        use reqwest::{Request, Response, Url};

        use crate::{
            arweave::{visitor::arweave_visitor, Arweave, ArweaveContext},
            context::ArweaveAccess,
            http::{reqwest::mock::MockHttpClient, ClientAccess},
        };

        #[derive(Clone)]
        struct Context {
            arweave: Arweave,
            http_client: MockHttpClient,
        }

        impl Context {
            pub fn new(arweave: Arweave, http_client: MockHttpClient) -> Self {
                Context {
                    http_client,
                    arweave,
                }
            }
        }

        impl ClientAccess<MockHttpClient> for Context {
            fn get_http_client(&self) -> &MockHttpClient {
                &self.http_client
            }
        }

        impl ArweaveContext<MockHttpClient> for Context {
            fn get_client(&self) -> &MockHttpClient {
                &self.http_client
            }
        }

        impl ArweaveAccess for Context {
            fn arweave(&self) -> &Arweave {
                &self.arweave
            }
        }

        #[tokio::test]
        async fn stream_blocks_with_arweave_visitor() {
            let http_client = {
                MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
                    .when(|req: &Request| {
                        req.url().as_str() == "http://example.com/info"
                    })
                    .then(|_: &Request| {
                        let data = "{\"network\":\"arweave.N.1\",\"version\":5,\"release\":53,\"height\":1039304,\"current\":\"NoD90XIXNVUvFVUW66EvJyjYelwrw7QM94HQ5MnvVwOODxgVtGqfwZgYBcw7Okv6\",\"blocks\":1039305,\"peers\":5077,\"queue_length\":0,\"node_state_latency\":1}";
                        let response = http::response::Builder::new()
                            .status(200)
                            .body(data)
                            .unwrap();
                        Response::from(response)
                    })
                    .when(move |req: &Request| {
                        req.url().as_str().ends_with(
                            "NoD90XIXNVUvFVUW66EvJyjYelwrw7QM94HQ5MnvVwOODxgVtGqfwZgYBcw7Okv6",
                        )
                    })
                    .then(move |_| {
                        let data = fs::read_to_string("./test-data/block_NoD90XIXNVUvFVUW66EvJyjYelwrw7QM94HQ5MnvVwOODxgVtGqfwZgYBcw7Okv6.json").expect("Failed to read test data");
                        let response = http::response::Builder::new()
                            .status(200)
                            .body(data)
                            .unwrap();
                        Response::from(response)
                    })
                    .when(move |req: &Request| {
                        req.url().as_str().ends_with(
                            "Tn5fr-RL1L5PfXeWTrH7umebRx8RCChbSrBdB-Q5E3vJt8jgP6UnayltOyv2Zo-w",
                        )
                    })
                    .then(move |_| {
                        let data = fs::read_to_string("./test-data/block_Tn5fr-RL1L5PfXeWTrH7umebRx8RCChbSrBdB-Q5E3vJt8jgP6UnayltOyv2Zo-w.json").expect("Failed to read test data");
                        let response = http::response::Builder::new()
                            .status(200)
                            .body(data)
                            .unwrap();
                        Response::from(response)
                    })
                    .when(move |req: &Request| {
                        req.url().as_str().ends_with(
                            "nbu76VJa4p-Y7HXp8OKALSQpHinRne-LJVPH9rpUHjxKCKi3y5XJisUlAtE3IBET",
                        )
                    })
                    .then(move |_| {
                        let data = fs::read_to_string("./test-data/block_nbu76VJa4p-Y7HXp8OKALSQpHinRne-LJVPH9rpUHjxKCKi3y5XJisUlAtE3IBET.json").expect("Failed to read test data");
                        let response = http::response::Builder::new()
                            .status(200)
                            .body(data)
                            .unwrap();
                        Response::from(response)
                    })
            };

            let arweave = Arweave::new(Url::from_str("http://example.com").unwrap());
            let ctx = Context::new(arweave, http_client);

            let current_head = ctx.arweave().get_network_info(&ctx).await.unwrap().current;

            let visitor = arweave_visitor(&ctx, current_head);
            pin_mut!(visitor);

            assert_eq!(
                visitor
                    .next()
                    .await
                    .expect("Expected to find next block, but got none")
                    .expect("Request for the block resulted with an error")
                    .indep_hash,
                "NoD90XIXNVUvFVUW66EvJyjYelwrw7QM94HQ5MnvVwOODxgVtGqfwZgYBcw7Okv6"
            );
            assert_eq!(
                visitor
                    .next()
                    .await
                    .expect("Expected to find next block, but got none")
                    .expect("Request for the block resulted with an error")
                    .indep_hash,
                "Tn5fr-RL1L5PfXeWTrH7umebRx8RCChbSrBdB-Q5E3vJt8jgP6UnayltOyv2Zo-w"
            );
            assert_eq!(
                visitor
                    .next()
                    .await
                    .expect("Expected to find next block, but got none")
                    .expect("Request for the block resulted with an error")
                    .indep_hash,
                "nbu76VJa4p-Y7HXp8OKALSQpHinRne-LJVPH9rpUHjxKCKi3y5XJisUlAtE3IBET"
            );
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Address(String);

impl Address {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<str> for Address {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl FromStr for Address {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl TryFrom<Owner> for Address {
    type Error = Infallible; // FIXME: use proper error

    fn try_from(o: Owner) -> Result<Self, Self::Error> {
        public_key_to_address(o.0.as_bytes())
    }
}

impl TryFrom<&Owner> for Address {
    type Error = Infallible; // FIXME: use proper error

    fn try_from(o: &Owner) -> Result<Self, Self::Error> {
        public_key_to_address(o.0.as_bytes())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BlockHash(String);

impl BlockHash {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BlockHeight(u128);

impl fmt::Display for BlockHeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<u128> for BlockHeight {
    fn eq(&self, other: &u128) -> bool {
        self.0 == *other
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct BlockIndepHash(String);

impl BlockIndepHash {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for BlockIndepHash {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(BlockIndepHash(s.to_owned()))
    }
}

impl From<&str> for BlockIndepHash {
    fn from(s: &str) -> Self {
        BlockIndepHash(s.to_owned())
    }
}

impl From<String> for BlockIndepHash {
    fn from(s: String) -> Self {
        BlockIndepHash(s)
    }
}

impl fmt::Display for BlockIndepHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
impl PartialEq<&str> for BlockIndepHash {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TransactionId(String);

impl TransactionId {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for TransactionId {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TransactionId(s.to_owned()))
    }
}

impl From<&str> for TransactionId {
    fn from(s: &str) -> Self {
        TransactionId(s.to_owned())
    }
}

impl From<String> for TransactionId {
    fn from(s: String) -> Self {
        TransactionId(s)
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Owner(String);

impl Owner {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<&str> for Owner {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

pub mod tags {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::TagName;

    pub fn serialize<S>(name: TagName, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = data_encoding::BASE64URL.encode(name.0.as_bytes());
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TagName, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = data_encoding::BASE64URL.decode(s.as_bytes()).unwrap(); // FIXME: do not unwrap, map error
        Ok(TagName(String::from_utf8(s).unwrap())) // FIXME: do not unwrap, map error
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct TagName(String);

impl TagName {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TagName {
    fn from(v: &str) -> Self {
        Self(v.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TagValue(String);

impl TagValue {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for TagValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TagValue {
    fn from(v: &str) -> Self {
        Self(v.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
    name: TagName,
    value: TagValue,
}

impl From<(&str, &str)> for Tag {
    fn from((name, value): (&str, &str)) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl PartialEq<(&'static str, &'static str)> for Tag {
    fn eq(&self, other: &(&str, &str)) -> bool {
        self.name.0 == other.0 && self.value.0 == other.1
    }
}

impl PartialEq<Tag> for (&'static str, &'static str) {
    fn eq(&self, other: &Tag) -> bool {
        self.0 == other.name.0 && self.1 == other.value.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Signature(String);

impl Signature {
    pub fn to_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String")]
pub struct TransactionSize(usize);

impl FromStr for TransactionSize {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TransactionSize(s.parse()?))
    }
}

impl From<TransactionSize> for u64 {
    fn from(v: TransactionSize) -> Self {
        v.0 as u64
    }
}

impl TryFrom<String> for TransactionSize {
    type Error = ParseIntError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl fmt::Display for TransactionSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkInfo {
    pub network: String,
    pub version: usize,
    pub release: usize,
    pub height: u128,
    pub current: BlockIndepHash,
    pub blocks: usize,
    pub peers: usize,
    pub queue_length: usize,
    pub node_state_latency: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockInfo {
    pub hash: BlockHash,
    pub height: BlockHeight,
    pub indep_hash: BlockIndepHash,
    pub previous_block: BlockIndepHash,
    pub tx_root: TransactionId,
    pub txs: Vec<TransactionId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub last_tx: TransactionId,
    pub data_size: TransactionSize,
    pub owner: Owner,
    pub tags: Vec<Tag>,
    pub signature: Signature,
}

pub mod serde_stringify {
    use std::str::FromStr;

    use serde::{de, Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>, R>(deserializer: D) -> Result<R, D::Error>
    where
        R: FromStr,
        R::Err: ToString,
    {
        let s: &str = de::Deserialize::deserialize(deserializer)?;
        s.parse()
            .map_err(|err: R::Err| de::Error::custom(err.to_string()))
    }

    pub fn serialize<S, T>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: ToString,
    {
        serializer.serialize_str(&val.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Offset {
    #[serde(with = "serde_stringify")]
    offset: u64,
    #[serde(with = "serde_stringify")]
    size: u64,
}

#[derive(Clone, Debug, Deserialize)]
struct Chunk<'a> {
    chunk: &'a [u8],
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Node(String);

impl Node {
    pub fn new(host_and_port: String) -> Self {
        Self(host_and_port)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

// From<&str> is only available for tests, otherwise use TryFrom
#[cfg(test)]
impl From<&str> for Node {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl TryFrom<&Url> for Node {
    type Error = url::ParseError;
    fn try_from(url: &Url) -> Result<Self, Self::Error> {
        match (url.host(), url.port()) {
            (Some(host), Some(port)) => Ok(Node(format!("{}:{}", host, port))),
            (Some(host), None) => Ok(Node(host.to_string())),
            (None, _) => Err(url::ParseError::EmptyHost),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PeerProcessingStatus {
    Pending,
    Ok,
    Failed,
}

async fn get<HttpClient>(
    client: &HttpClient,
    url: Url,
    timeout: Option<Duration>,
) -> Result<HttpClient::Response, ArweaveError>
where
    HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
    HttpClient::Error: From<reqwest::Error>,
{
    let uri = Uri::from_str(url.as_ref()).map_err(|err| {
        error!("Failed to translate Url to Uri: {:?}", err);
        ArweaveError::UnknownErr
    })?;

    let req: http::Request<String> = http::request::Builder::new()
        .method(http::Method::GET)
        .uri(uri)
        .body("".to_string())
        .map_err(|err| {
            error!(
                "Failed to build request for fetching transaction info: {:?}",
                err
            );
            ArweaveError::MalformedRequest
        })?;

    let mut req: reqwest::Request = reqwest::Request::try_from(req).map_err(|err| {
        error!(
            "Failed to translate http::Request to reqwest::Request: {:?}",
            err
        );
        ArweaveError::UnknownErr
    })?;

    if let Some(timeout) = timeout {
        *req.timeout_mut() = Some(timeout);
    }

    client.execute(req).await.map_err(|err| {
        debug!("Request failed: {:?}", err);
        ArweaveError::RequestFailed
    })
}

#[derive(Debug)]
enum FetchPeersError<HttpClientError> {
    HttpClientError(HttpClientError),
    ArweaveError(ArweaveError),
    UnsupportedPeerAddress(Node),
    ResponseDeserializationError,
}

fn get_peers<HttpClient>(
    client: HttpClient,
    node: Node,
    timeout: Option<Duration>,
) -> BoxFuture<'static, Result<Vec<Node>, FetchPeersError<HttpClient::Error>>>
where
    HttpClient:
        Client<Request = reqwest::Request, Response = reqwest::Response> + Send + Sync + 'static,
    HttpClient::Error: From<reqwest::Error>,
{
    async move {
        debug!("Get peers for {}", node);
        let url =
            match Url::from_str(&format!("http://{}", node)).and_then(|base| base.join("/peers")) {
                Ok(url) => url,
                Err(err) => {
                    debug!(
                        "Failed to build URL request peers for node: {}, {:?}",
                        node, err
                    );
                    return Err(FetchPeersError::UnsupportedPeerAddress(node.clone()));
                }
            };

        let res = match get(&client, url, timeout).await {
            Ok(res) => match res.error_for_status() {
                Ok(res) => res,
                Err(err) => {
                    debug!(
                        "Request for fetching peers failed, peer: {}, err: {:?}",
                        node, err
                    );
                    return Err(FetchPeersError::HttpClientError(err.into()));
                }
            },
            Err(err) => {
                debug!(
                    "Request for fetching peers failed, peer: {}, err: {:?}",
                    node, err
                );
                return Err(FetchPeersError::ArweaveError(err));
            }
        };

        let peers: Vec<Node> = match res.json().await {
            Ok(peers) => peers,
            Err(err) => {
                debug!(
                    "Failed to deserialize peers, peer: {}, error: {:?}",
                    node, err
                );
                return Err(FetchPeersError::ResponseDeserializationError);
            }
        };

        Ok(peers)
    }
    .boxed()
}

#[derive(Clone, Debug, Display, Eq, Error, PartialEq)]
pub enum ArweaveError {
    MalformedRequest,
    RequestFailed,
    UnknownErr,
}

impl From<anyhow::Error> for ArweaveError {
    fn from(_err: anyhow::Error) -> ArweaveError {
        ArweaveError::UnknownErr
    }
}

#[derive(Clone)]
pub enum ArweaveProtocol {
    Http,
    Https,
}

#[derive(Clone)]
pub struct Arweave {
    base_url: Url,
}

pub trait ArweaveContext<HttpClient>: ClientAccess<HttpClient>
where
    HttpClient: crate::http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    fn get_client(&self) -> &HttpClient;
}

impl Arweave {
    pub fn new(url: Url) -> Arweave {
        Arweave { base_url: url }
    }

    pub async fn get_network_info<Context, HttpClient>(
        &self,
        ctx: &Context,
    ) -> Result<NetworkInfo, ArweaveError>
    where
        Context: ClientAccess<HttpClient>,
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
        HttpClient::Error: From<reqwest::Error>,
    {
        info!("Fetch network info");
        let url = self.base_url.join("/info").map_err(|err| {
            error!("Failed to build request Url: {:?}", err);
            ArweaveError::MalformedRequest
        })?;
        let uri = Uri::from_str(url.as_ref()).map_err(|err| {
            error!("Failed to translate Url to Uri: {:?}", err);
            ArweaveError::UnknownErr
        })?;

        let req: http::Request<String> = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(uri)
            .body("".to_string())
            .map_err(|err| {
                error!(
                    "Failed to build request for fetching network info: {:?}",
                    err
                );
                ArweaveError::MalformedRequest
            })?;

        let req: reqwest::Request = reqwest::Request::try_from(req).map_err(|err| {
            error!(
                "Failed to translate http::Request to reqwest::Request: {:?}",
                err
            );
            ArweaveError::UnknownErr
        })?;

        let client = ctx.get_http_client();
        let res = execute_with_retry::<tokio::runtime::Handle, _>(client, 3, req)
            .await
            .map_err(|err| {
                error!("Request failed: {:?}", err);
                ArweaveError::RequestFailed
            })?;

        match res.error_for_status() {
            Ok(res) => res.json().await.map_err(|err| {
                error!("Request failed: {}", err);
                ArweaveError::RequestFailed
            }),
            Err(err) => {
                error!("Request failed {}", err);
                Err(ArweaveError::RequestFailed)
            }
        }
    }

    pub async fn get_block_info<Context, HttpClient>(
        &self,
        ctx: &Context,
        block_id: &BlockIndepHash,
    ) -> Result<BlockInfo, ArweaveError>
    where
        Context: ClientAccess<HttpClient>,
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
        HttpClient::Error: From<reqwest::Error>,
    {
        info!("Fetch block info for {}", block_id);

        let url = self
            .base_url
            // TODO: double check that url encoding is not needed for block hash
            .join(&format!("/block/hash/{}", &block_id))
            .map_err(|err| {
                error!("Failed to build request Url: {:?}", err);
                ArweaveError::MalformedRequest
            })?;

        let uri = Uri::from_str(url.as_ref()).map_err(|err| {
            error!("Failed to translate Url to Uri: {:?}", err);
            ArweaveError::UnknownErr
        })?;

        let req: http::Request<String> = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(uri)
            .body("".to_string())
            .map_err(|err| {
                error!("Failed to build request for fetching block info: {:?}", err);
                ArweaveError::MalformedRequest
            })?;

        let req: reqwest::Request = reqwest::Request::try_from(req).map_err(|err| {
            error!(
                "Failed to translate http::Request to reqwest::Request: {:?}",
                err
            );
            ArweaveError::UnknownErr
        })?;

        let client = ctx.get_http_client();
        let res = execute_with_retry::<tokio::runtime::Handle, _>(client, 3, req)
            .await
            .map_err(|err| {
                error!("Request failed: {:?}", err);
                ArweaveError::RequestFailed
            })?;

        match res.error_for_status() {
            Ok(res) => res.json().await.map_err(|err| {
                error!("Request failed: {}", err);
                ArweaveError::RequestFailed
            }),
            Err(err) => {
                error!("Request failed {}", err);
                Err(ArweaveError::RequestFailed)
            }
        }
    }

    pub async fn get_transaction_info<Context, HttpClient>(
        &self,
        ctx: &Context,
        transaction_id: &TransactionId,
    ) -> Result<Transaction, ArweaveError>
    where
        Context: ClientAccess<HttpClient>,
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
        HttpClient::Error: From<reqwest::Error>,
    {
        info!("Fetch transaction info for {}", transaction_id);

        let url = self
            .base_url
            // TODO: double check that url encoding is not needed for block hash
            .join(&format!("/tx/{}", &transaction_id))
            .map_err(|err| {
                error!("Failed to build request Url: {:?}", err);
                ArweaveError::MalformedRequest
            })?;

        let uri = Uri::from_str(url.as_ref()).map_err(|err| {
            error!("Failed to translate Url to Uri: {:?}", err);
            ArweaveError::UnknownErr
        })?;

        let req: http::Request<String> = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(uri)
            .body("".to_string())
            .map_err(|err| {
                error!(
                    "Failed to build request for fetching transaction info: {:?}",
                    err
                );
                ArweaveError::MalformedRequest
            })?;

        let req: reqwest::Request = reqwest::Request::try_from(req).map_err(|err| {
            error!(
                "Failed to translate http::Request to reqwest::Request: {:?}",
                err
            );
            ArweaveError::UnknownErr
        })?;

        let client = ctx.get_http_client();
        let res = execute_with_retry::<tokio::runtime::Handle, _>(client, 3, req)
            .await
            .map_err(|err| {
                error!("Request failed: {:?}", err);
                ArweaveError::RequestFailed
            })?;

        match res.error_for_status() {
            Ok(res) => res.json().await.map_err(|err| {
                error!("Failed to deserialize response data: {}", err);
                ArweaveError::RequestFailed
            }),
            Err(err) => {
                error!("Request failed {}", err);
                Err(ArweaveError::RequestFailed)
            }
        }
    }

    pub async fn download_transaction_data<Context, HttpClient, Output>(
        &self,
        ctx: &Context,
        tx: &TransactionId,
        output: &mut Output,
        peer: Option<Url>,
    ) -> Result<(), ArweaveError>
    where
        Context: ClientAccess<HttpClient>,
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
        HttpClient::Error: From<reqwest::Error>,
        Output: AsyncWrite + AsyncSeek + Unpin,
    {
        let client = ctx.get_http_client();
        let base_url = if let Some(ref peer) = peer {
            peer
        } else {
            &self.base_url
        };
        let url = base_url
            .join(&format!("/tx/{}/offset", &tx))
            .map_err(|err| {
                error!("Failed to build request Url: {:?}", err);
                ArweaveError::MalformedRequest
            })?;
        let res: reqwest::Response = get(client, url, None).await?;
        let Offset { offset, size } = res.json().await.map_err(|err| {
            error!("Failed to parse response for offset data: {:?}", err);
            ArweaveError::UnknownErr
        })?;

        info!("Transaction offset={}, size={}", offset, size);

        let end_offset = offset;
        let start_offset = offset - size + 1;
        let mut chunk_offset = start_offset;
        while chunk_offset < end_offset + 1 {
            let url = self
                .base_url
                .join(&format!("/chunk/{}", &chunk_offset))
                .map_err(|err| {
                    error!("Failed to build request Url: {:?}", err);
                    ArweaveError::MalformedRequest
                })?;

            let mut res = get(client, url, None).await?;

            let content_length: u64 = res
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| {
                    error!("Could not read chunk size, missing Content-Length header");
                    ArweaveError::RequestFailed
                })?;

            let file_offset = chunk_offset - start_offset;

            // This is not strictly needed at the moment, but keeping as
            // an example, because this will be needed once we add retry
            // and concurrency
            output
                .seek(std::io::SeekFrom::Start(file_offset))
                .await
                .map_err(|err| {
                    error!(
                        "Failed to seek into the right position in the output: {:?}",
                        err
                    );
                    ArweaveError::RequestFailed
                })?;

            let mut buf = Vec::with_capacity(content_length as usize);
            while let Some(chunk) = res.chunk().await.map_err(|err| {
                error!("Failed to read chunk data: {:?}", err);
                ArweaveError::RequestFailed
            })? {
                buf.write_all(&chunk).await.map_err(|err| {
                    error!("Failed to write chunk data to output: {:?}", err);
                    ArweaveError::RequestFailed
                })?;
            }
            let chunk: Chunk = serde_json::from_slice(buf.as_slice()).unwrap();
            let data = BASE64URL_NOPAD.decode(chunk.chunk).unwrap();

            output.write_all(data.as_slice()).await.map_err(|err| {
                error!("Failed to write chunk data: {:?}", err);
                ArweaveError::UnknownErr
            })?;

            let chunk_size = data.len() as u64;

            info!(
                "Got chunk: offset={}, chunk_size={}",
                chunk_offset, chunk_size
            );

            chunk_offset += chunk_size;
        }

        Ok(())
    }

    pub async fn find_nodes<Context, HttpClient>(
        &self,
        ctx: &Context,
        concurrency_level: u16,
        timeout: Duration,
        max_depth: Option<usize>,
        max_count: Option<usize>,
    ) -> Result<Vec<Node>, ArweaveError>
    where
        Context: ClientAccess<HttpClient>,
        HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>
            + Send
            + Sync
            + Clone
            + 'static,
        HttpClient::Error: From<reqwest::Error>,
    {
        let max_count = max_count.unwrap_or(100);
        let max_depth = max_depth.unwrap_or(3);
        let http_client = ctx.get_http_client().clone();
        let concurrency_level = concurrency_level as usize;

        info!(
            "Find nodes, max_depth={}, max_count={}, req_timeout={:?}, concurrency_level={}",
            max_depth, max_count, timeout, concurrency_level
        );

        let cache: Arc<Mutex<HashMap<Node, PeerProcessingStatus>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let gateway_node = match (self.base_url.domain(), self.base_url.port()) {
            (Some(domain), Some(port)) => Ok(Node(format!("{}:{}", domain, port))),
            (Some(domain), None) => Ok(Node(domain.to_string())),
            (None, _) => {
                error!("Domain for Arweave gateway's URL cannot be empty");
                Err(ArweaveError::UnknownErr)
            }
        }?;

        let unchecked_nodes: Vec<(Node, usize)> =
            get_peers(http_client.clone(), gateway_node.clone(), Some(timeout))
                .await
                .map(|peers| peers.into_iter().map(|peer| (peer, 0)).collect())
                .map_err(|err| match err {
                    FetchPeersError::HttpClientError(_) => ArweaveError::RequestFailed,
                    FetchPeersError::ArweaveError(err) => err,
                    FetchPeersError::UnsupportedPeerAddress(_) => ArweaveError::MalformedRequest,
                    FetchPeersError::ResponseDeserializationError => ArweaveError::UnknownErr,
                })?;

        {
            let mut cache = cache.lock().expect("Failed to acquire lock");

            cache.insert(gateway_node.clone(), PeerProcessingStatus::Ok);

            unchecked_nodes.iter().for_each(|(node, _)| {
                cache.insert(node.clone(), PeerProcessingStatus::Pending);
            });
        }

        let unchecked_nodes = DynamicAsyncQueue::new(unchecked_nodes);
        let busy_jobs = Arc::new(AtomicU16::new(0));
        let unchecked_nodes_notifier = unchecked_nodes.clone();

        let good_nodes = {
            let cache = cache.clone();
            pin_mut!(unchecked_nodes_notifier);
            pin_mut!(busy_jobs);
            pin_mut!(cache);
            pin_mut!(http_client);

            unchecked_nodes
                .map(|(node, depth)| {
                    busy_jobs.fetch_add(1, Ordering::Relaxed);
                    info!("Fetch peers for node={}, depth={}", node, depth,);
                    get_peers(http_client.clone(), node.clone(), Some(timeout))
                        .map(move |res| (node, depth, res))
                })
                .buffer_unordered(concurrency_level)
                .filter_map(|(node, depth, res)| {
                    let cache = cache.clone();
                    let unchecked_nodes_notifier = unchecked_nodes_notifier.clone();
                    let busy_jobs = busy_jobs.clone();
                    async move {
                        let ret = match res {
                            Ok(peers) => {
                                let mut cache = cache.lock().expect("Failed to acquire lock");

                                *cache
                                    .get_mut(&node)
                                    .expect("Failed to find node from cache") =
                                    PeerProcessingStatus::Ok;

                                if depth < max_depth {
                                    let new_nodes: Vec<(Node, usize)> = peers
                                        .iter()
                                        .filter(|peer| !cache.contains_key(peer))
                                        .cloned()
                                        .map(|node| (node, depth + 1))
                                        .collect();

                                    new_nodes.iter().for_each(|(node, _)| {
                                        cache.insert(node.clone(), PeerProcessingStatus::Pending);
                                    });

                                    info!(
                                        "Found good node {}, with {} peers, {} new",
                                        node,
                                        peers.len(),
                                        new_nodes.len()
                                    );

                                    unchecked_nodes_notifier.add_items(new_nodes);
                                } else {
                                    info!(
                                        "Found good node {}, with {} peers, maximum depth reached",
                                        node,
                                        peers.len()
                                    );
                                }
                                Some((node, peers))
                            }
                            Err(_err) => {
                                let mut status = cache.lock().expect("Failed to acquire lock");
                                *status
                                    .get_mut(&node)
                                    .expect("Failed to find node from cache") =
                                    PeerProcessingStatus::Failed;
                                None
                            }
                        };
                        let busy_jobs = busy_jobs.fetch_sub(1, Ordering::Relaxed);
                        // busy_jobs here is the previous value before decrementing
                        if busy_jobs < 2 {
                            unchecked_nodes_notifier.all_pending_work_done();
                        }
                        ret
                    }
                })
                .take(max_count)
                .collect::<Vec<(Node, Vec<Node>)>>()
                .await
        };

        let mut cache = Arc::try_unwrap(cache)
            .expect("Cache not freed yet, while should be")
            .into_inner()
            .expect("Failed to unwrap mutex");

        // Remove gateway from the returned results
        cache.remove(&gateway_node);

        info!(
            "found {} nodes, {} good",
            cache.keys().len(),
            good_nodes.len()
        );

        Ok(good_nodes.into_iter().fold(vec![], |mut acc, (node, _)| {
            acc.push(node);
            acc
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs::File,
        str::FromStr,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use crate::{
        arweave::{Arweave, Node},
        context::test_utils::test_context_with_http_client,
        http::{mock::Call, reqwest::mock::MockHttpClient},
        key_manager::test_utils::test_keys,
    };
    use http::Method;
    use reqwest::{Request, Response};
    use url::Url;

    use super::{Offset, Transaction, TransactionSize};

    #[actix_rt::test]
    async fn get_network_info() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/info";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = "{\"network\":\"arweave.N.1\",\"version\":5,\"release\":43,\"height\":551511,\"current\":\"XIDpYbc3b5iuiqclSl_Hrx263Sd4zzmrNja1cvFlqNWUGuyymhhGZYI4WMsID1K3\",\"blocks\":97375,\"peers\":64,\"queue_length\":0,\"node_state_latency\":18}";
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            });

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client.clone());
        let arweave = Arweave::new(Url::from_str("http://example.com").unwrap());
        let network_info = arweave.get_network_info(&ctx).await.unwrap();

        // release other references to the client.
        drop(ctx);

        assert_eq!(network_info.height, 551511);

        // Double check that we only made single HTTP request
        client.verify(|calls| {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].count(), 1);
        });
    }

    #[actix_rt::test]
    async fn get_network_info_is_tried_thrice() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/info";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = "{\"network\":\"arweave.N.1\",\"version\":5,\"release\":43,\"height\":551511,\"current\":\"XIDpYbc3b5iuiqclSl_Hrx263Sd4zzmrNja1cvFlqNWUGuyymhhGZYI4WMsID1K3\",\"blocks\":97375,\"peers\":64,\"queue_length\":0,\"node_state_latency\":18}";
                let response = http::response::Builder::new()
                    .status(500)
                    .body(data)
                    .unwrap();
                Response::from(response)
            });

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client.clone());
        let arweave = Arweave::new(Url::from_str("http://example.com").unwrap());

        assert!(arweave.get_network_info(&ctx).await.is_err());

        // release other references to the client.
        drop(ctx);

        // Make sure we end up trying three times before failing
        client.verify(|calls| {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].count(), 3);
        });
    }

    #[actix_rt::test]
    async fn get_block_info() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/block/hash/R_8ZquUbAOjm9ESBRaA2IkVCQbdrXG771xtPrU2wgydsQ0SgkBTN8NMltWxLL17Y";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = r#"{"usd_to_ar_rate":["11522","640909"],"scheduled_usd_to_ar_rate":["7237","400568"],"packing_2_5_threshold":"0","strict_data_split_threshold":"30607159107830","nonce":"XHTUAFD3qtjdImvskmmVQfeiGGyBoGGYT3IDRHBIO6A","previous_block":"66wdorN5h6SvnD80m7UFefFSTaL2G4H1LHRnUp09AlBCs_F5n8dhce02bUw2qWGZ","timestamp":1661931861,"last_retarget":1661931861,"diff":"115792089195331436664126265822706335936480533844803212669378321398735515654742","height":1006670,"hash":"_____ub07xinRnVu2qL61lik0KgC7QZh71jIWROrzl0","indep_hash":"R_8ZquUbAOjm9ESBRaA2IkVCQbdrXG771xtPrU2wgydsQ0SgkBTN8NMltWxLL17Y","txs":["UfTkJlndiiVd9eEuLD8jdIy97TuLYi0dUYxeb6-Z6wM"],"tx_root":"eWMTraCmjupOiEpmuFHlPHJD31U7VfYXIyIDuaQeRhU","wallet_list":"9qKSNAsSwYjHdhB_cj5jeanB2I-O0IrqkPDFPddma6cXaL97kavyoMmedyogFpt5","reward_addr":"n-BT67MKIwO7tAjcTUsxflje7xHtJ9Xe0akVmNiQw0Y","tags":[],"reward_pool":"40092847901462759","weave_size":"97088718807286","block_size":"262144","cumulative_diff":"3890700788977390","hash_list_merkle":"FnEmRv7xxe7_X7uN2uHWpSLBbO9m1VyWjQ5w3ozrmhDauDqLz6_iGpign75iIhFa","poa":{"option":"1","tx_path":"TRUNCATED","data_path":"TRUNCATED","chunk":"TRUNCATED"}}"#;
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            });

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client.clone());
        let arweave = Arweave::new(Url::from_str("http://example.com").unwrap());
        let block_info = arweave
            .get_block_info(
                &ctx,
                &"R_8ZquUbAOjm9ESBRaA2IkVCQbdrXG771xtPrU2wgydsQ0SgkBTN8NMltWxLL17Y".into(),
            )
            .await
            .unwrap();

        assert_eq!(block_info.height, 1006670);
        assert_eq!(
            block_info.txs,
            ["UfTkJlndiiVd9eEuLD8jdIy97TuLYi0dUYxeb6-Z6wM".into()]
        )
    }

    #[actix_rt::test]
    async fn get_transaction_info() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/block/hash/R_8ZquUbAOjm9ESBRaA2IkVCQbdrXG771xtPrU2wgydsQ0SgkBTN8NMltWxLL17Y";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = r#"{"usd_to_ar_rate":["11522","640909"],"scheduled_usd_to_ar_rate":["7237","400568"],"packing_2_5_threshold":"0","strict_data_split_threshold":"30607159107830","nonce":"XHTUAFD3qtjdImvskmmVQfeiGGyBoGGYT3IDRHBIO6A","previous_block":"66wdorN5h6SvnD80m7UFefFSTaL2G4H1LHRnUp09AlBCs_F5n8dhce02bUw2qWGZ","timestamp":1661931861,"last_retarget":1661931861,"diff":"115792089195331436664126265822706335936480533844803212669378321398735515654742","height":1006670,"hash":"_____ub07xinRnVu2qL61lik0KgC7QZh71jIWROrzl0","indep_hash":"R_8ZquUbAOjm9ESBRaA2IkVCQbdrXG771xtPrU2wgydsQ0SgkBTN8NMltWxLL17Y","txs":["UfTkJlndiiVd9eEuLD8jdIy97TuLYi0dUYxeb6-Z6wM"],"tx_root":"eWMTraCmjupOiEpmuFHlPHJD31U7VfYXIyIDuaQeRhU","wallet_list":"9qKSNAsSwYjHdhB_cj5jeanB2I-O0IrqkPDFPddma6cXaL97kavyoMmedyogFpt5","reward_addr":"n-BT67MKIwO7tAjcTUsxflje7xHtJ9Xe0akVmNiQw0Y","tags":[],"reward_pool":"40092847901462759","weave_size":"97088718807286","block_size":"262144","cumulative_diff":"3890700788977390","hash_list_merkle":"FnEmRv7xxe7_X7uN2uHWpSLBbO9m1VyWjQ5w3ozrmhDauDqLz6_iGpign75iIhFa","poa":{"option":"1","tx_path":"TRUNCATED","data_path":"TRUNCATED","chunk":"TRUNCATED"}}"#;
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            });

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client.clone());
        let arweave = Arweave::new(Url::from_str("http://example.com").unwrap());
        let block_info = arweave
            .get_block_info(
                &ctx,
                &"R_8ZquUbAOjm9ESBRaA2IkVCQbdrXG771xtPrU2wgydsQ0SgkBTN8NMltWxLL17Y".into(),
            )
            .await
            .unwrap();

        assert_eq!(block_info.height, 1006670);
        assert_eq!(
            block_info.txs,
            ["UfTkJlndiiVd9eEuLD8jdIy97TuLYi0dUYxeb6-Z6wM".into()]
        )
    }

    #[test]
    #[allow(non_snake_case)]
    fn deserialize_transaction_RpwDyKqv1Z9J2H8ky2bwyGteIio3Mhqbhb0eYCJMHkc() {
        let tx_id = "RpwDyKqv1Z9J2H8ky2bwyGteIio3Mhqbhb0eYCJMHkc";
        let mut tx_data_file = project_root::get_project_root().unwrap();
        tx_data_file.push("test-data");
        tx_data_file.push(format!("{}.json", tx_id));
        let data = File::open(tx_data_file).unwrap();
        let _: Transaction = serde_json::from_reader(data).unwrap();
    }

    #[test]
    fn serialize_transaction_size() {
        let json = serde_json::to_string(&TransactionSize(1)).unwrap();

        assert_eq!(json, "1")
    }

    #[test]
    fn deserialize_transaction_offset() {
        let offset: Offset =
            serde_json::from_str(r#"{"size":"680081503","offset":"97934916049237"}"#).unwrap();

        assert_eq!(
            offset,
            Offset {
                offset: 97934916049237,
                size: 680081503
            }
        )
    }

    #[test]
    fn fetch_chunks() {
        // let offset = get_transaction_offset(tx);
        // const size = offset.size;
        // const endOffset = offset.offset;
        // const startOffset = endOffset - size + 1;
    }

    #[actix_rt::test]
    async fn find_nodes() {
        let test_data: Arc<std::sync::Mutex<HashMap<Node, Vec<Node>>>> =
            Arc::new(Mutex::new(HashMap::from([
                (
                    "example.com".into(),
                    vec![
                        "1.202.113.98:1984".into(),
                        "47.252.4.63:8700".into(),
                        "171.117.206.105:1984".into(),
                        "140.224.64.87:1984".into(),
                        "110.87.132.19:1984".into(),
                    ],
                ),
                (
                    "1.202.113.98:1984".into(),
                    vec![
                        "47.252.4.63:8700".into(),
                        "171.117.206.105:1984".into(),
                        "140.224.64.87:1984".into(),
                        "110.87.132.19:1984".into(),
                    ],
                ),
                (
                    "47.252.4.63:8700".into(),
                    vec![
                        "1.202.113.98:1984".into(),
                        "171.117.206.105:1984".into(),
                        "140.224.64.87:1984".into(),
                        "110.87.132.19:1984".into(),
                    ],
                ),
                (
                    "171.117.206.105:1984".into(),
                    vec![
                        "1.202.113.98:1984".into(),
                        "47.252.4.63:8700".into(),
                        "140.224.64.87:1984".into(),
                        "110.87.132.19:1984".into(),
                    ],
                ),
                (
                    "140.224.64.87:1984".into(),
                    vec![
                        "1.202.113.98:1984".into(),
                        "47.252.4.63:8700".into(),
                        "171.117.206.105:1984".into(),
                        "110.87.132.19:1984".into(),
                    ],
                ),
            ])));

        let client = {
            let test_data_1 = test_data.clone();
            let test_data_2 = test_data.clone();
            MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
                .when(move |req: &Request| {
                    if let Ok(ref node) = req.url().try_into() {
                        (node == &"110.87.132.19:1984".into()
                            || test_data_1
                                .lock()
                                .expect("Failed to get lock")
                                .contains_key(node))
                            && req.url().path() == "/peers"
                            && req.method() == http::Method::GET
                    } else {
                        false
                    }
                })
                .then(move |req: &Request| {
                    let node: Node = req.url().try_into().unwrap();
                    if node == "110.87.132.19:1984".into() {
                        let response = http::response::Builder::new()
                            .status(http::StatusCode::BAD_GATEWAY)
                            .body("".to_string())
                            .unwrap();
                        return Response::from(response);
                    }
                    let data = serde_json::to_string(
                        test_data_2
                            .lock()
                            .unwrap()
                            .get(&node)
                            .expect(&format!("Unexpected request for peers, node={}", node)),
                    )
                    .unwrap();
                    let response = http::response::Builder::new()
                        .status(200)
                        .body(data)
                        .unwrap();
                    Response::from(response)
                })
        };

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client.clone());
        let arweave = Arweave::new(Url::from_str("http://example.com").unwrap());

        let mut nodes = arweave
            .find_nodes(&ctx, 10, Duration::from_secs(1), None, None)
            .await
            .unwrap();

        drop(ctx);

        let mut expected = Vec::<Node>::from([
            "1.202.113.98:1984".into(),
            "47.252.4.63:8700".into(),
            "171.117.206.105:1984".into(),
            "140.224.64.87:1984".into(),
        ]);

        expected.sort();
        nodes.sort();
        assert_eq!(nodes, expected);

        client.verify(|calls| {
            assert_eq!(calls.len(), 6);
            calls
                .iter()
                .for_each(|Call { req: _, count }| assert_eq!(*count, 1))
        });
    }
}
