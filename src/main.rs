mod server;
mod cron;

use server::run_server;
use cron::run_cron;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().unwrap();
    tokio::task::spawn(run_cron());
    run_server().await
}