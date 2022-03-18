mod error;
pub(crate) mod routes;

use std::net::SocketAddr;

use actix_web::{
    middleware::Logger,
    web::{self, Data},
    App, HttpServer,
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    SqliteConnection,
};
use paris::info;
use routes::get_tx::get_tx;
use routes::index::index;
use routes::post_tx::post_tx;

use crate::{key_manager, server::routes::sign::sign_route, state::ValidatorStateAccess};

pub trait RuntimeContext {
    fn bind_address(&self) -> &SocketAddr;
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>>;
    fn redis_connection_url(&self) -> &str;
}

pub async fn run_server<Context, KeyManager>(ctx: Context) -> std::io::Result<()>
where
    Context: RuntimeContext
        + routes::sign::Config<KeyManager>
        + ValidatorStateAccess
        + Clone
        + Send
        + 'static,
    KeyManager: key_manager::KeyManager + Clone + Send + 'static,
{
    env_logger::init();

    info!("Starting up HTTP server...");

    let server_config = ctx.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(server_config.clone()))
            .wrap(Logger::default())
            .route("/", web::get().to(index))
            .route("/tx/{tx_id}", web::get().to(get_tx::<Context>))
            .service(
                web::scope("/cosigner")
                    .route("/sign", web::post().to(sign_route::<Context, KeyManager>)),
            )
            .service(
                web::scope("/leader").route("/tx", web::post().to(post_tx::<Context, KeyManager>)),
            )
            .service(web::scope("/idle").route("/", web::get().to(index)))
    })
    .shutdown_timeout(5)
    .bind(ctx.bind_address())?
    .run()
    .await
}
