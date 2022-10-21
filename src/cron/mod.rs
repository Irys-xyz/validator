mod arweave;
mod bundle;
mod clear_transactions;
mod contract;
mod error;
mod slasher;
mod transactions;
mod validate;

use crate::{
    context,
    contract_gateway::{self, ContractGatewayError},
    database::queries,
    http, key_manager,
};
use derive_more::{Display, Error};
use futures::{join, Future};
use log::{error, info};
use std::time::Duration;

use self::error::ValidatorCronError;
use crate::arweave::{ArweaveContext, ArweaveError};

#[derive(Debug, Display, Error, Clone, PartialEq)]
pub enum CronJobError {
    ArweaveError(ArweaveError),
    ContractGatewayError(ContractGatewayError),
    ValidatorError(ValidatorCronError),
    NetworkSyncError,
    Other,
}

// Update contract state
pub async fn run_crons<Context, HttpClient, KeyManager>(ctx: Context)
where
    Context: ArweaveContext<HttpClient>
        + context::ArweaveAccess
        + context::BundlerAccess
        + context::ValidatorAddressAccess
        + contract_gateway::ContractGatewayAccess
        + http::ClientAccess<HttpClient>
        + key_manager::KeyManagerAccess<KeyManager>
        + queries::QueryContext
        + Unpin,
    HttpClient: http::Client<
            Request = reqwest::Request,
            Response = reqwest::Response,
            Error = reqwest::Error,
        > + Clone
        + Send
        + Sync
        + 'static,
    HttpClient::Error: Send,
    KeyManager: key_manager::KeyManager,
{
    info!("Validator starting ...");
    join!(
        create_cron(
            &ctx,
            "check contract updates",
            contract::check_contract_updates,
            30
        ),
        create_cron(&ctx, "sync network info", arweave::sync_network_info, 30),
        // create_cron(&ctx, "validate bundler", validate::validate, 2 * 60),
        create_cron(
            &ctx,
            "sync network info",
            validate::validate_transactions,
            30
        ),
        create_cron(
            &ctx,
            "clear old transactions",
            clear_transactions::clear_old_transactions,
            180
        )
    );
}

async fn create_cron<'a, Context, F>(
    ctx: &'a Context,
    description: &str,
    f: impl Fn(&'a Context) -> F,
    sleep: u64,
) where
    F: Future<Output = Result<(), CronJobError>> + 'a,
{
    loop {
        info!("Task running - {}", description);
        match f(ctx).await {
            Ok(_) => info!("Task finished - {}", description),
            Err(e) => error!("Task error - {} with {}", description, e),
        };

        info!("Task sleeping for {} seconds - {}", sleep, description);
        tokio::time::sleep(Duration::from_secs(sleep)).await;
    }
}
