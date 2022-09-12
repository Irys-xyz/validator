extern crate diesel;

use super::error::ValidatorCronError;
use super::slasher::vote_slash;
use super::transactions::get_transactions;
use crate::arweave::{self, ArweaveContext};
use crate::arweave::{Arweave, Transaction as ArweaveTx};
use crate::bundler::Bundler;
use crate::context::{ArweaveAccess, BundlerAccess};
use crate::database::models::{Block, Epoch, NewBundle, NewTransaction};
use crate::database::queries::{self, *};
use crate::http::{self, Client};
use crate::key_manager;
use crate::key_manager::KeyManagerAccess;
use crate::types::Validator;
use bundlr_sdk::deep_hash_sync::{deep_hash_sync, ONE_AS_BUFFER};
use bundlr_sdk::verify::types::Item;
use bundlr_sdk::{deep_hash::DeepHashChunk, verify::file::verify_file_bundle};
use data_encoding::BASE64URL_NOPAD;
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct TxReceipt {
    block: u128,
    tx_id: String,
    signature: String,
}

pub async fn validate_bundler<Context, HttpClient, KeyManager>(
    ctx: &Context,
) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext
        + arweave::ArweaveContext<HttpClient>
        + ArweaveAccess
        + BundlerAccess
        + KeyManagerAccess<KeyManager>
        + http::ClientAccess<HttpClient>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
    KeyManager: key_manager::KeyManager,
{
    todo!()
}

async fn validate_bundle<Context, HttpClient, KeyManager>(
    ctx: &Context,
    arweave: &Arweave,
    bundle: &ArweaveTx,
) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext
        + ArweaveContext<HttpClient>
        + BundlerAccess
        + KeyManagerAccess<KeyManager>
        + http::ClientAccess<HttpClient>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
    KeyManager: key_manager::KeyManager,
{
    todo!()
}

fn check_bundle_block(bundle: &ArweaveTx) -> Result<Option<u128>, ValidatorCronError> {
    todo!()
}

fn store_bundle<Context>(
    ctx: &Context,
    bundle: &ArweaveTx,
    current_block: u128,
) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext + BundlerAccess,
{
    todo!()
}

async fn verify_bundle_tx<Context, HttpClient, KeyManager>(
    ctx: &Context,
    bundle_tx: &Item,
    current_block: Option<u128>,
) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext + KeyManagerAccess<KeyManager> + http::ClientAccess<HttpClient>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
    KeyManager: key_manager::KeyManager,
{
    todo!()
}

async fn tx_exists_on_peers<Context, HttpClient>(
    ctx: &Context,
    tx_id: &str,
) -> Result<TxReceipt, ValidatorCronError>
where
    Context: http::ClientAccess<HttpClient>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    todo!()
}

fn verify_tx_receipt<KeyManager>(
    key_manager: &KeyManager,
    tx_receipt: &TxReceipt,
) -> std::io::Result<bool>
where
    KeyManager: key_manager::KeyManager,
{
    todo!()
}

pub async fn validate_transactions<HttpClient>(
    http_client: &HttpClient,
    bundler: &Bundler,
) -> Result<(), ValidatorCronError>
where
    HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    todo!()
}

#[cfg(test)]
mod tests {
    use crate::utils::get_file_as_byte_vector;
    use crate::{
        context::test_utils::test_context_with_http_client, http::reqwest::mock::MockHttpClient,
        key_manager::test_utils::test_keys,
    };
    use http::Method;
    use reqwest::{Request, Response};

    use super::validate_bundler;

    #[actix_rt::test]
    async fn validate_bundler_should_abort_due_no_block() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/graphql?query=query%28%24owners%3A%20%5BString%21%5D%2C%20%24first%3A%20Int%29%20%7B%20transactions%28owners%3A%20%24owners%2C%20first%3A%20%24first%29%20%7B%20pageInfo%20%7B%20hasNextPage%20%7D%20edges%20%7B%20cursor%20node%20%7B%20id%20owner%20%7B%20address%20%7D%20signature%20recipient%20tags%20%7B%20name%20value%20%7D%20block%20%7B%20height%20id%20timestamp%20%7D%20%7D%20%7D%20%7D%20%7D";
                req.method() == Method::POST && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = "{\"data\": {\"transactions\": {\"pageInfo\": {\"hasNextPage\": true },\"edges\": [{\"cursor\": \"cursor\", \"node\": { \"id\": \"tx_id\",\"owner\": {\"address\": \"address\"}, \"signature\": \"signature\",\"recipient\": \"\", \"tags\": [], \"block\": null } } ] } } }";
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            })
            .when(|req: &Request| {
                let url = "http://example.com/tx_id";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = "";
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            });

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client);
        let res = validate_bundler(&ctx).await;
        assert!(res.is_ok())
    }

    #[actix_rt::test]
    async fn validate_bundler_should_return_ok() {
        let client = MockHttpClient::new(|a: &Request, b: &Request| a.url() == b.url())
            .when(|req: &Request| {
                let url = "http://example.com/graphql?query=query%28%24owners%3A%20%5BString%21%5D%2C%20%24first%3A%20Int%29%20%7B%20transactions%28owners%3A%20%24owners%2C%20first%3A%20%24first%29%20%7B%20pageInfo%20%7B%20hasNextPage%20%7D%20edges%20%7B%20cursor%20node%20%7B%20id%20owner%20%7B%20address%20%7D%20signature%20recipient%20tags%20%7B%20name%20value%20%7D%20block%20%7B%20height%20id%20timestamp%20%7D%20%7D%20%7D%20%7D%20%7D";
                req.method() == Method::POST && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let data = "{\"data\": {\"transactions\": {\"pageInfo\": {\"hasNextPage\": true },\"edges\": [{\"cursor\": \"cursor\", \"node\": { \"id\": \"tx_id\",\"owner\": {\"address\": \"address\"}, \"signature\": \"signature\", \"recipient\": \"\", \"tags\": [], \"block\": { \"id\": \"id\", \"timestamp\": 10, \"height\": 10 } } } ] } } }";
                let response = http::response::Builder::new()
                    .status(200)
                    .body(data)
                    .unwrap();
                Response::from(response)
            })
            .when(|req: &Request| {
                let url = "http://example.com/tx_id";
                req.method() == Method::GET && &req.url().to_string() == url
            })
            .then(|_: &Request| {
                let buffer = get_file_as_byte_vector("./bundles/test_bundle").unwrap();
                let response = http::response::Builder::new()
                    .status(200)
                    .body(buffer)
                    .unwrap();
                Response::from(response)
            });

        let (key_manager, _bundle_pvk) = test_keys();
        let ctx = test_context_with_http_client(key_manager, client);
        let res = validate_bundler(&ctx).await;
        assert!(res.is_ok())
    }
}
