mod bundle;
mod error;
mod validate;
mod contract;
mod arweave;

use std::time::Duration;

use futures::Future;
use paris::{info, error};

use self::contract::update_contract;

// Update contract state
pub async fn run_crons() {
    create_cron("", validate::validate, 10);
}

fn create_cron<F>(description: &'static str, f: impl Fn() -> F + 'static, sleep: u64) 
where
    F: Future + 'static,
    F::Output: 'static
{
    tokio::task::spawn_local(async move {
        loop {
            if let Err(e) = update_contract().await {
                error!("Error occurred while {} - {}", description, e);
                panic!("{}", e);
            };

            f().await;

            tokio::time::sleep(Duration::from_secs(sleep)).await;
        };
    });
}