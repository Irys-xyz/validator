use std::pin::Pin;

use clap::Parser;

use env_logger::Env;
use futures::{
    stream::{self, StreamExt, TryStreamExt},
    TryFutureExt,
};
use serde::{Deserialize, Serialize};
use url::Url;
use validator::{
    arweave::{
        visitor::arweave_visitor, Address, Arweave, ArweaveContext, BlockHeight, BlockIndepHash,
        BlockInfo, Transaction, TransactionId, TransactionSize,
    },
    bundlr::tags::{BUNDLE_ACTION_TAG, BUNDLR_APP_TAG},
    context::ArweaveAccess,
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

#[derive(Clone)]
struct AppContext {
    http_client: ReqwestClient,
    arweave: Arweave,
}

impl AppContext {
    pub fn new(http_client: reqwest::Client, arweave: Arweave) -> Self {
        AppContext {
            http_client: ReqwestClient::new(http_client),
            arweave,
        }
    }
}

impl ClientAccess<ReqwestClient> for AppContext {
    fn get_http_client(&self) -> &ReqwestClient {
        &self.http_client
    }
}

impl ClientAccess<ReqwestClient> for Pin<Box<AppContext>> {
    fn get_http_client(&self) -> &ReqwestClient {
        &self.http_client
    }
}

impl ArweaveContext<ReqwestClient> for AppContext {
    fn get_client(&self) -> &ReqwestClient {
        &self.http_client
    }
}

impl ArweaveContext<ReqwestClient> for Pin<Box<AppContext>> {
    fn get_client(&self) -> &ReqwestClient {
        &self.http_client
    }
}

impl ArweaveAccess for AppContext {
    fn arweave(&self) -> &Arweave {
        &self.arweave
    }
}

impl ArweaveAccess for Pin<Box<AppContext>> {
    fn arweave(&self) -> &Arweave {
        &self.arweave
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BlockRecord {
    id: BlockIndepHash,
    parent: BlockIndepHash,
    height: BlockHeight,
    transactions: Vec<TxRecord>,
}

impl BlockRecord {
    fn new(block: BlockInfo, transactions: Vec<Transaction>) -> Self {
        Self {
            id: block.indep_hash,
            parent: block.previous_block,
            height: block.height,
            transactions: transactions.iter().map(Into::into).collect(),
        }
    }
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
    let ctx = Box::pin(AppContext::new(reqwest::Client::default(), arweave));

    let start_block = if let Some(start_block) = args.start_block {
        start_block
    } else {
        let arweave = ctx.arweave();
        let network_info = arweave
            .get_network_info(&ctx)
            .await
            .expect("Failed to fetch network info");
        network_info.current
    };

    arweave_visitor(&ctx, start_block)
        .take(args.depth)
        .map_ok(|block| {
            // A bit annoying, but we need to take a ref that can be moved because we
            // need those inner async blocks otherwise lifetimes get too complicated
            // or maybe I'm just missing something obvious
            let ctx = &ctx;
            async move {
                stream::iter(block.txs.clone())
                    .map(|tx| async move { ctx.arweave().get_transaction_info(ctx, &tx).await })
                    .buffered(10) // when fetching the data, do 10 parallel requests
                    .try_fold(
                        Vec::with_capacity(block.txs.len()),
                        |mut acc, tx| async move {
                            acc.push(tx);
                            Ok(acc)
                        },
                    )
                    .and_then(|txs| futures::future::ready(Ok(BlockRecord::new(block, txs))))
                    .await
            }
        })
        .try_buffered(10)
        .try_for_each(|record| async move {
            println!(
                "{}",
                serde_json::to_string(&record).expect("Failed to serialize block record as JSON")
            );

            Ok(())
        })
        .await
        .expect("Failed");
}
