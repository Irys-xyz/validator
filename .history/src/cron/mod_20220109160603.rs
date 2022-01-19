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
    loop {
        if let Err(e) = update_contract().await {
            error!("Error occurred while updating contract state - {}", e);
            panic!("{}", e);
        };

        validate::validate().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2 * 60)).await;
    };
}

async fn create_cron(fut: impl Future<Output=()>, sleep: u64) {
    tokio::spawn(fut);
}