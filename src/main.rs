#[macro_use]
extern crate diesel;

mod bundle;
mod consts;
mod cron;
mod database;
mod server;
mod types;

use cron::run_crons;
use server::run_server;
use std::collections::HashSet;

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().unwrap();
    std::env::set_var("RUST_LOG", "RUST_LOG=info,sqlx=warn,a=debug");

    let mut set = HashSet::new();
    for arg in std::env::args() {
        set.insert(arg);
    }

    if !set.contains("--no-cron") {
        paris::info!("Running with cron");
        tokio::task::spawn_local(run_crons());
    } else {
    };

    if !set.contains("--no-server") {
        paris::info!("Running with server");
        run_server().await.unwrap()
    };
}
