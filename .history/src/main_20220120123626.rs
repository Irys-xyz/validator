
#[macro_use]
extern crate diesel;

mod server;
mod cron;
mod bundle;
mod database;
mod types;
mod consts;

use server::run_server;
use cron::run_crons;

#[actix_web::main]
async fn main() -> () {
    println!("{}", std::env::args())
    dotenv::dotenv().unwrap();
    tokio::task::spawn_local(run_crons());
    run_server().await.unwrap()
}