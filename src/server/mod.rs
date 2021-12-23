mod routes;
mod error;

use std::sync::RwLock;

use actix_web::{HttpServer, App, web::{self, Data}, middleware::Logger};
use paris::info;
use routes::post_tx::post_tx;
use routes::index::index;

struct State {
    x: String
}

impl State {
    pub fn new() -> Self {
        State { x: "hi".to_string() }
    }
}

pub async fn run_server() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "RUST_LOG=info,sqlx=warn,a=debug");
    env_logger::init();
    let port = std::env::var("PORT").map(|s| s.parse::<u16>().unwrap()).unwrap_or(10000);
    let redis_connection_string = std::env::var("REDIS_CONNECTION_URL").unwrap();
    let redis_client = redis::Client::open(redis_connection_string.as_str()).unwrap();

    let state = RwLock::new(State::new());

    info!("Starting up HTTP server...");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(redis_client.clone()))
            .wrap(Logger::default())
            .route("/", web::get().to(index))
            .route("/tx", web::post().to(post_tx))
    })
    .bind(format!("127.0.0.1:{}", port))?
    .run()
    .await
}