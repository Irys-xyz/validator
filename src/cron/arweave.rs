use futures::{pin_mut, Future};
use log::{debug, error};

use crate::arweave::{self, ArweaveError, BlockIndepHash, BlockInfo};
use crate::context;
use crate::cron::CronJobError;
use crate::http::Client;
use crate::state::ValidatorStateAccess;

pub type StorageError = ();

async fn is_known_block<Context>(ctx: &Context, block_info: &BlockInfo) -> bool {
    false
}

async fn store_block_info<Context>(
    ctx: &Context,
    block_info: BlockInfo,
) -> Result<(), StorageError> {
    Ok(())
}

async fn traverse_blockchain<Context, BlockHandler, BlockHandlerFuture, HttpClient>(
    ctx: &Context,
    start_block: BlockIndepHash,
    block_handler: BlockHandler,
) -> Result<(), CronJobError>
where
    Context: context::ArweaveAccess + arweave::ArweaveContext<HttpClient>,
    BlockHandler: Fn(&Context, usize, BlockInfo) -> BlockHandlerFuture,
    BlockHandlerFuture: Future<Output = bool>,
    HttpClient: Client<Request = reqwest::Request, Response = reqwest::Response>,
    HttpClient::Error: From<reqwest::Error>,
{
    let arweave = ctx.arweave();

    let mut block = start_block;
    let mut depth = 0;

    loop {
        let block_info = arweave
            .get_block_info(ctx, &block)
            .await
            .map_err(|err| CronJobError::ArweaveError(err))?;

        block = block_info.previous_block.clone();

        if !block_handler(ctx, depth, block_info).await {
            break;
        }

        depth += 1;
    }

    return Ok(());
}

pub async fn sync_network_info<Context, HttpClient>(ctx: &Context) -> Result<(), CronJobError>
where
    Context: arweave::ArweaveContext<HttpClient> + context::ArweaveAccess + ValidatorStateAccess,
    HttpClient: crate::http::Client<Request = reqwest::Request, Response = reqwest::Response>,
    HttpClient::Error: From<reqwest::Error>,
{
    let network_info = ctx.arweave().get_network_info(ctx).await.map_err(|err| {
        error!("Request for network info failed: {:?}", err);
        CronJobError::ArweaveError(ArweaveError::UnknownErr)
    })?;

    let state = ctx.get_validator_state();

    let head = network_info.current;

    traverse_blockchain(ctx, head, |ctx, depth, block| async move {
        debug!(
            "Blockchain traversal, depth={}, block: height={}, id={}",
            depth, block.height, block.indep_hash
        );
        depth < 10
    });

    Ok(())
}
