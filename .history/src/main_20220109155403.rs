#![feature(async_stream)]
#![feature(slice_pattern)]
#![feature(slice_as_chunks)]

#[macro_use]
extern crate diesel;

mod server;
mod cron;
mod bundle;
mod database;
mod types;

use server::run_server;
use cron::run_crons;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().unwrap();
    run_crons();
    run_server().await
}