#[macro_use]
extern crate diesel;

mod bundle;
mod consts;
mod cron;
mod database;
mod server;
mod state;
mod types;

use clap::Parser;
use cron::run_crons;
use server::{run_server, ServerConfig};
use state::generate_state;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
struct AppConfig {
    /// Do not start cron jobs
    #[clap(long)]
    no_cron: bool,

    /// Do not start app in server mode
    #[clap(long)]
    no_server: bool,

    /// Database connection URL
    #[clap(long, env, default_value = "postgres://bundlr:bundlr@127.0.0.1/bundlr")]
    database_url: String,

    /// Redis connection URL
    #[clap(long, env, default_value = "redis://127.0.0.1")]
    redis_connection_url: String,

    /// Listen address for the server
    #[clap(short, long, env, default_value = "127.0.0.1:10000")]
    listen: SocketAddr,
}

impl ServerConfig for AppConfig {
    fn database_connection_url(&self) -> &str {
        &self.database_url
    }

    fn redis_connection_url(&self) -> &str {
        &self.redis_connection_url
    }

    fn bind_address(&self) -> &SocketAddr {
        &self.listen
    }
}

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().ok();

    let config = AppConfig::parse();
    let state = generate_state();

    if !config.no_cron {
        paris::info!("Running with cron");
        tokio::task::spawn_local(run_crons(state));
    };

    if !config.no_server {
        paris::info!("Running with server");
        run_server(&config).await.unwrap()
    };
}
