mod server;
mod cron;
mod bundle;
mod types;
mod database;

use server::run_server;
use cron::run_crons;

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().unwrap();
    tokio::task::spawn_local(run_crons());
    run_server().await.unwrap()
}