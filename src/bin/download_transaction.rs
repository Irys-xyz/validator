use std::path::PathBuf;

use clap::Parser;

use env_logger::Env;
use log::info;
use tokio::fs::File;
use url::Url;
use validator::{
    arweave::{Arweave, ArweaveContext, ArweaveError, TransactionId},
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
async fn main() -> Result<(), ArweaveError> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();

    let arweave = Arweave::new(args.arweave_gateway.clone());
    let ctx = Context::new(reqwest::Client::default());

    let mut tx_data_file = args.tx_cache.clone();
    tx_data_file.push(format!("{}", args.tx));
    // let tx_data_file = fs::canonicalize(tx_data_file)
    //     .expect("Failed to create absolute file path from provided information");

    info!("Write tx data to {:?}", tx_data_file);

    let mut output = File::create(tx_data_file)
        .await
        .expect("Failed to open file for writing");

    arweave
        .download_transaction_data(&ctx, &args.tx, &mut output, None)
        .await
}
