use futures::stream::TryStreamExt;
use futures::{Future, TryFutureExt};
use log::error;

use crate::arweave::visitor::arweave_visitor;
use crate::arweave::{self, ArweaveError, BlockInfo};
use crate::context;
use crate::cron::CronJobError;
use crate::state::ValidatorStateAccess;

pub type StorageError = ();

fn is_new_block<Context>(
    _ctx: &Context,
    _block: &BlockInfo,
) -> impl Future<Output = Result<bool, StorageError>>
where
    Context: Unpin,
{
    futures::future::ready(Ok(true))
}

fn process_new_block<Context>(
    _ctx: &Context,
    _block_info: BlockInfo,
) -> impl Future<Output = Result<(), StorageError>>
where
    Context: Unpin,
{
    futures::future::ready(Ok(()))
}

pub async fn sync_network_info<Context, HttpClient>(ctx: &Context) -> Result<(), CronJobError>
where
    Context:
        arweave::ArweaveContext<HttpClient> + context::ArweaveAccess + ValidatorStateAccess + Unpin,
    HttpClient: crate::http::Client<Request = reqwest::Request, Response = reqwest::Response>
        + Clone
        + Send
        + Sync
        + 'static,
    HttpClient::Error: From<reqwest::Error> + Send,
{
    let network_info = ctx.arweave().get_network_info(ctx).await.map_err(|err| {
        error!("Request for network info failed: {:?}", err);
        CronJobError::ArweaveError(ArweaveError::UnknownErr)
    })?;

    let visitor = arweave_visitor(ctx, network_info.current);

    visitor
        .map_err(|err| {
            error!("Failed to get next block: {:?}", err);
            CronJobError::NetworkSyncError
        })
        .try_take_while(|block| {
            is_new_block(&ctx, &block).map_err(|err| {
                error!("Check if block is new failed: {:?}", err);
                CronJobError::NetworkSyncError
            })
        })
        .try_for_each(|block| {
            process_new_block(ctx, block).map_err(|err| {
                error!("Request for network info failed: {:?}", err);
                CronJobError::NetworkSyncError
            })
        })
        .await
}

#[cfg(test)]
mod tests {}
