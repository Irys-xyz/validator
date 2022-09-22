use std::{
    fs::{self, DirBuilder},
    path::PathBuf,
};

use clap::Parser;

use env_logger::Env;
use log::info;
use tokio::{fs::File, io::AsyncWriteExt};
use url::Url;
use validator::{
    arweave::{Arweave, ArweaveContext, TransactionId},
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
        &self.http_client
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

    let mut tx_data_file = args.tx_cache.clone();
    tx_data_file.push(format!("{}.data", args.tx));

    let mut tx_metadata_file = args.tx_cache;
    tx_metadata_file.push(format!("{}.json", args.tx));

    let tx_metadata = arweave
        .get_transaction_info(&ctx, &args.tx)
        .await
        .expect("Failed to fetch transaction info");

    let mut metadata_file = File::create(tx_metadata_file.clone())
        .await
        .expect("Failed to open metadata file for writing");

    let mut data_file = File::create(tx_data_file.clone())
        .await
        .expect("Failed to open data file for writing");

    data_file
        .set_len(tx_metadata.data_size.clone().into())
        .await
        .expect("Failed to reserve space for data file");

    arweave
        .download_transaction_data(&ctx, 16, &args.tx, &mut data_file, None, None)
        .await
        .expect("Failed to download transaction data");

    let json = serde_json::to_string(&tx_metadata).expect("Failed to serialize tx metadata");
    metadata_file
        .write_all(json.as_bytes())
        .await
        .expect("Failed to write metadata file");

    let metadata_file_path = fs::canonicalize(tx_metadata_file)
        .expect("Failed to create absolute file path from provided information");

    let data_file_path = fs::canonicalize(tx_data_file)
        .expect("Failed to create absolute file path from provided information");

    info!(
        "Wrote tx data to {} and metadat to {}",
        data_file_path.to_str().unwrap(),
        metadata_file_path.to_str().unwrap()
    );
}
