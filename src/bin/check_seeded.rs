use std::time::Duration;

use clap::Parser;
use log::info;

use env_logger::Env;
use url::Url;
use validator::{
    arweave::{Arweave, ArweaveContext, TransactionId},
    http::{reqwest::ReqwestClient, ClientAccess},
};

#[derive(Parser)]
struct Args {
    #[clap(long, env = "ARWEAVE_GATEWAY", default_value = "https://arweave.net")]
    arweave_gateway: Url,

    #[clap(long)]
    tx: TransactionId,

    #[clap(long, default_value = "10")]
    req_timeout_secs: u64,

    #[clap(long, default_value = "100")]
    max_concurrency: u16,

    #[clap(long)]
    max_depth: Option<usize>,

    #[clap(long)]
    max_count: Option<usize>,
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

    let seeded = arweave.is_seeded(
        &ctx,
        args.tx,
        5,
        args.max_concurrency,
        Duration::from_secs(args.req_timeout_secs),
        args.max_depth,
        args.max_count)
        .await;

    info!("Seeded: {:?}", seeded);
}
