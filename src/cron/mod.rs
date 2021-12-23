use std::time::Duration;

use paris::{info};

// Update contract state
pub async fn run_cron() {
    loop {
        info!("hello");
        tokio::time::sleep(Duration::from_secs(2 * 60)).await;
    }
}