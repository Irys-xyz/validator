pub mod arweave;
mod bundle;
mod contract;
mod error;
mod leader;
mod slasher;
mod state;
mod transactions;
mod validate;

use self::{error::ValidatorCronError, state::SharedValidatorState};
use crate::cron::state::generate_state;
use futures::{join, Future};
use paris::{error, info};
use std::time::Duration;

// Update contract state
pub async fn run_crons() {
    let state = generate_state();

    info!("Validator starting ...");
    join!(
        //create_cron("update contract", contract::update_contract, 30),
        create_cron("validate bundler", validate::validate, 2 * 60, &state),
        create_cron(
            "validate transactions",
            validate::validate_transactions,
            30,
            &state
        ),
        create_cron(
            "send transactions to leader",
            leader::send_txs_to_leader,
            60,
            &state
        )
    );
}

async fn create_cron<F>(
    description: &'static str,
    f: impl Fn(SharedValidatorState) -> F,
    sleep: u64,
    shared_state: &SharedValidatorState,
) where
    F: Future<Output = Result<(), ValidatorCronError>>,
    F::Output: 'static,
{
    loop {
        info!("Task running - {}", description);
        match f(shared_state.clone()).await {
            Ok(_) => info!("Task finished - {}", description),
            Err(e) => error!("Task error - {} with {}", description, e),
        };

        info!("Task sleeping for {} seconds - {}", sleep, description);
        tokio::time::sleep(Duration::from_secs(sleep)).await;
    }
}
