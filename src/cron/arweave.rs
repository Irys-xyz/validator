use log::error;
use log::info;

use crate::arweave::ArweaveContext;
use crate::arweave::ArweaveError;
use crate::context::ArweaveAccess;
use crate::cron::CronJobError;
use crate::state::ValidatorStateAccess;

pub async fn sync_network_info<Context, HttpClient>(ctx: &Context) -> Result<(), CronJobError>
where
    Context: ArweaveContext<HttpClient> + ArweaveAccess + ValidatorStateAccess,
    HttpClient: crate::http::Client<Request = reqwest::Request, Response = reqwest::Response>,
    HttpClient::Error: From<reqwest::Error>,
{
    let network_info = ctx.arweave().get_network_info(ctx).await.map_err(|err| {
        error!("Request for network info failed: {:?}", err);
        CronJobError::ArweaveError(ArweaveError::UnknownErr)
    })?;

    let state = ctx.get_validator_state();

    info!("Update state: current_block={}", network_info.height);
    state.set_current_block(network_info.height);

    Ok(())
}
