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
use crate::cron::state::{AtomicValidatorState, ValidatorState};
use futures::{join, Future};
use lazy_static::lazy_static;
use paris::{error, info};
use std::sync::Arc;
use std::time::Duration;

// Update contract state
pub async fn run_crons() {
    lazy_static! {
        static ref VALIDATOR_STATE: SharedValidatorState =
            Arc::new(AtomicValidatorState::new(ValidatorState::Cosigner));
    }

    info!("Validator starting ...");
    join!(
        //create_cron("update contract", contract::update_contract, 30),
        create_cron(
            "validate bundler",
            validate::validate,
            2 * 60,
            &VALIDATOR_STATE
        ),
        create_cron(
            "validate transactions",
            validate::validate_transactions,
            30,
            &VALIDATOR_STATE
        ),
        create_cron(
            "send transactions to leader",
            leader::send_txs_to_leader,
            60,
            &VALIDATOR_STATE
        )
    );
}

async fn create_cron<F>(
    description: &'static str,
    f: impl Fn(&'static SharedValidatorState) -> F + 'static,
    sleep: u64,
    shared_state: &'static SharedValidatorState,
) where
    F: Future<Output = Result<(), ValidatorCronError>> + 'static,
    F::Output: 'static,
{
    loop {
        info!("Task running - {}", description);
        match f(shared_state).await {
            Ok(_) => info!("Task finished - {}", description),
            Err(e) => error!("Task error - {} with {}", description, e),
        };

        info!("Task sleeping for {} seconds - {}", sleep, description);
        tokio::time::sleep(Duration::from_secs(sleep)).await;
    }
}
