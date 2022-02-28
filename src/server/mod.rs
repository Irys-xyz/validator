mod error;
mod routes;

use std::net::SocketAddr;

use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpResponse, HttpServer,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use paris::info;
use reool::RedisPool;
use routes::get_tx::get_tx;
use routes::index::index;
use routes::post_tx::post_tx;
use tokio::runtime::Handle;

use crate::server::routes::sign::sign_route;

pub trait ServerConfig {
    fn bind_address(&self) -> &SocketAddr;
    fn database_connection_url(&self) -> &str;
    fn redis_connection_url(&self) -> &str;
}

pub async fn run_server<Config>(config: &Config) -> std::io::Result<()>
where
    Config: ServerConfig,
{
    env_logger::init();

    let redis_connection_string = config.redis_connection_url().to_string();
    let db_url = config.database_connection_url().to_string();
    info!("Starting HTTP server...");

    HttpServer::new(move || {
        let conn_manager = ConnectionManager::<PgConnection>::new(db_url.clone());

        let redis_pool = RedisPool::builder()
            .connect_to_node(redis_connection_string.clone())
            .desired_pool_size(5)
            .task_executor(Handle::current())
            .finish_redis_rs()
            .unwrap();

        let postgres_pool = Pool::builder().max_size(10).build(conn_manager).unwrap();

        App::new()
            .app_data(Data::new(redis_pool))
            .app_data(Data::new(postgres_pool))
            .wrap(Logger::default())
            .route("/", web::get().to(index))
            .service(
                web::scope("/cosigner")
                    .route("/tx/{tx_id}", web::get().to(get_tx))
                    .route("/tx", web::post().to(post_tx))
                    .route("/sign", web::post().to(sign_route)),
            )
            .service(
                web::scope("/leader")
                    .route("/tx/{tx_id}", web::get().to(HttpResponse::Ok))
                    .route("/tx", web::post().to(HttpResponse::Ok))
                    .route("/sign", web::post().to(HttpResponse::Ok)),
            )
            .service(web::scope("/idle").route("/", web::get().to(index)))
    })
    .shutdown_timeout(5)
    .bind(config.bind_address())?
    .run()
    .await
}
