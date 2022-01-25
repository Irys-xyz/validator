mod routes;
mod error;

use std::sync::{RwLock, Arc};

use actix_web::{HttpServer, App, web::{self, Data}, middleware::Logger};
use paris::info;
use reool::RedisPool;
use routes::get_tx::get_tx;
use routes::post_tx::post_tx;
use routes::index::index;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tokio::runtime::Handle;

use crate::server::routes::sign::sign;

pub async fn run_server() -> std::io::Result<()> {
    info!("Starting up HTTP server...");

    std::env::set_var("RUST_LOG", "RUST_LOG=info,sqlx=warn,a=debug");
    env_logger::init();
    info!("Starting up HTTP server...");

    let port = std::env::var("PORT").map(|s| s.parse::<u16>().unwrap()).unwrap_or(10000);
    let redis_connection_string = std::env::var("REDIS_CONNECTION_URL").unwrap();
    info!("Starting up HTTP server...");

    let redis_client = redis::Client::open(redis_connection_string.as_str()).unwrap();

    let db_url = std::env::var("DATABASE_URL").unwrap();

    let validators = Arc::new(RwLock::new(Vec::<String>::new()));

    let v = validators.clone();
    actix_rt::spawn(async {
        let writer = v.write().unwrap();
        writer.
    });

    info!("Starting up HTTP server...");

    let pool = SqlitePoolOptions::new()
        .min_connections(10)
        .max_connections(100)
        .connect(&db_url)
        .await
        .expect("Couldn't connect to sqlite");

    HttpServer::new(move || {
        let redis_pool = RedisPool::builder()
            .connect_to_node(redis_connection_string.clone())
            .desired_pool_size(5)
            .task_executor(Handle::current())
            .finish_redis_rs()
            .unwrap();

        let postgres_pool = Pool::builder()
            .max_size(10)
            .build(conn_manager)
            .unwrap();

        App::new()
            .app_data(Data::from(validators.clone()))
            .app_data(Data::new(redis_pool))
            .app_data(Data::new(postgres_pool))
            .wrap(Logger::default())
            .route("/", web::get().to(index))
            .route("/tx/{tx_id}", web::get().to(get_tx))
            .route("/tx", web::post().to(post_tx))
            .route("/sign", web::post().to(sign))
    })
    .shutdown_timeout(5)
    .bind(format!("127.0.0.1:{}", port))?
    .run()
    .await
}