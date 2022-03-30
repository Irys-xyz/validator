pub mod arweave;
mod bundle;
mod contract;
mod error;
mod slasher;
mod transactions;
mod validate;

use crate::{database::queries, http};
use futures::{join, Future};
use paris::{error, info};
use std::{sync::Arc, time::Duration};

use self::error::ValidatorCronError;

// Update contract state
pub async fn run_crons<Context, HttpClient>(ctx: Context)
where
    Context: queries::QueryContext + arweave::ArweaveContext<HttpClient> + Clone,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    info!("Validator starting ...");
    join!(
        //create_cron("update contract", contract::update_contract, 30),
        create_cron(&ctx, "validate bundler", validate::validate, 2 * 60),
        create_cron(
            &ctx,
            "validate transactions",
            validate::validate_transactions,
            30
        ),
    );
}

async fn create_cron<'a, Context, F>(
    ctx: &Context,
    description: &'a str,
    f: impl Fn(Arc<Context>) -> F,
    sleep: u64,
) where
    Context: Clone + 'a,
    F: Future<Output = Result<(), ValidatorCronError>> + 'a,
{
    let ctx = Arc::new(ctx.clone());
    loop {
        info!("Task running - {}", description);
        match f(ctx.clone()).await {
            Ok(_) => info!("Task finished - {}", description),
            Err(e) => error!("Task error - {} with {}", description, e),
        };

        info!("Task sleeping for {} seconds - {}", sleep, description);
        tokio::time::sleep(Duration::from_secs(sleep)).await;
    }
}
