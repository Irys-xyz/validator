mod bundle;
mod error;
mod validate;
mod contract;
mod arweave;

use std::time::Duration;

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

pub async create_cron