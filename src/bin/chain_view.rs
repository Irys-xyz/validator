use clap::Parser;

use env_logger::Env;
use serde::{Deserialize, Serialize};
use url::Url;
use validator::{
    arweave::{
        Address, Arweave, ArweaveContext, BlockHeight, BlockIndepHash, BlockInfo, Transaction,
        TransactionId, TransactionSize,
    },
    bundler::tags::{BUNDLE_ACTION_TAG, BUNDLR_APP_TAG},
    http::{reqwest::ReqwestClient, ClientAccess},
};

#[derive(Parser)]
struct Args {
    #[clap(long, env = "ARWEAVE_GATEWAY", default_value = "https://arweave.net")]
    arweave_gateway: Url,
    #[clap(long, env = "DEPTH", default_value = "50")]
    depth: usize,
    #[clap(long)]
    start_block: Option<BlockIndepHash>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BlockRecord {
    id: BlockIndepHash,
    parent: BlockIndepHash,
    height: BlockHeight,
    transactions: Vec<TxRecord>,
}

impl From<(&BlockInfo, &[Transaction])> for BlockRecord {
    fn from((block_info, txs): (&BlockInfo, &[Transaction])) -> Self {
        let tx_records = txs.iter().map(|tx| tx.into()).collect::<Vec<TxRecord>>();
        Self {
            id: block_info.indep_hash.clone(),
            parent: block_info.previous_block.clone(),
            height: block_info.height.clone(),
            transactions: tx_records,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TxRecord {
    id: TransactionId,
    owner: Address,
    bundle: bool,
    size: TransactionSize,
}

impl From<&Transaction> for TxRecord {
    fn from(tx: &Transaction) -> Self {
        let is_bundle =
            tx.tags.contains(&BUNDLR_APP_TAG.into()) && tx.tags.contains(&BUNDLE_ACTION_TAG.into());
        Self {
            id: tx.id.clone(),
            owner: (&tx.owner)
                .try_into()
                .expect("Failed to translate transaction owner into address"),
            bundle: is_bundle,
            size: tx.data_size.clone(),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();

    let arweave = Arweave::new(args.arweave_gateway.clone());
    let ctx = Context::new(reqwest::Client::default());

    let start_block = if let Some(start_block) = args.start_block {
        start_block
    } else {
        let network_info = arweave
            .get_network_info(&ctx)
            .await
            .expect("Failed to fetch network info");
        network_info.current
    };

    let mut block_id = start_block;
    let mut depth = args.depth;
    while depth > 0 {
        let block_info = arweave
            .get_block_info(&ctx, &block_id)
            .await
            .expect("Failed to fetch block info");

        let mut txs = vec![];
        for tx_id in block_info.txs.iter() {
            let tx = arweave
                .get_transaction_info(&ctx, &tx_id)
                .await
                .expect("Failed to fetch transaction info");
            txs.push(tx)
        }

        let record = BlockRecord::from((&block_info, txs.as_slice()));

        println!(
            "{}",
            serde_json::to_string(&record).expect("Failed to serialize block record as JSON")
        );

        block_id = block_info.previous_block.clone();
        depth -= 1;
    }
}
