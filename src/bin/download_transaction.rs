use std::{
    fs::{self, DirBuilder},
    path::PathBuf,
};

use clap::Parser;

use env_logger::Env;
use log::info;
use tokio::fs::File;
use url::Url;
use validator::{
    arweave::{Arweave, ArweaveContext, Transaction, TransactionId},
    http::{reqwest::ReqwestClient, ClientAccess},
};

#[derive(Parser)]
struct Args {
    #[clap(long, env = "ARWEAVE_GATEWAY", default_value = "https://arweave.net")]
    arweave_gateway: Url,
    #[clap(long, env = "TX_CACHE", default_value = "./tx-cache/")]
    tx_cache: PathBuf,
    #[clap(long)]
    tx: TransactionId,
}

struct Context {
    http_client: ReqwestClient,
}

impl Context {
    pub fn new(http_client: reqwest::Client) -> Self {
        Context {
            http_client: ReqwestClient::new(http_client),
        }
    }
}

impl ClientAccess<ReqwestClient> for Context {
    fn get_http_client(&self) -> &ReqwestClient {
        &self.http_client
    }
}

impl ArweaveContext<ReqwestClient> for Context {
    fn get_client(&self) -> &ReqwestClient {
        &&self.http_client
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();

    let arweave = Arweave::new(args.arweave_gateway.clone());
    let ctx = Context::new(reqwest::Client::default());

    if !args
        .tx_cache
        .try_exists()
        .expect("Failed to check if tx cache folder exists")
    {
        DirBuilder::new()
            .recursive(true)
            .create(args.tx_cache.clone())
            .expect("Failed to create tx cache dir");
    }

    let mut tx_data_file = args.tx_cache;
    tx_data_file.push(format!("{}", args.tx));

    let Transaction { data_size, .. } = arweave
        .get_transaction_info(&ctx, &args.tx)
        .await
        .expect("Failed to fetch transaction size info");

    let mut output = File::create(tx_data_file.clone())
        .await
        .expect("Failed to open file for writing");

    output
        .set_len(data_size.into())
        .await
        .expect("Failed to set file size to match transaction data size");

    arweave
        .download_transaction_data(&ctx, 16, &args.tx, &mut output, vec![], Some(1))
        .await
        .expect("Failed to download transaction data");

    let file_path = fs::canonicalize(tx_data_file)
        .expect("Failed to create absolute file path from provided information");
    info!("Wrote tx data to {}", file_path.to_str().unwrap());
}
