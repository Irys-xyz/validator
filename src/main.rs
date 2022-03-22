#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod bundle;
mod consts;
mod cron;
mod database;
mod key_manager;
mod server;
mod state;
mod types;

use clap::Parser;
use cron::run_crons;
use database::queries;
use diesel::{
    r2d2::{self, ConnectionManager, PooledConnection},
    sqlite::SqliteConnection,
};
use jsonwebkey::{JsonWebKey, Key, PublicExponent, RsaPublic};
use key_manager::{InMemoryKeyManager, InMemoryKeyManagerConfig, KeyManager};
use server::{run_server, RuntimeContext};
use state::{generate_state, SharedValidatorState, ValidatorStateAccess};
use std::{fs, net::SocketAddr, sync::Arc};

embed_migrations!();

#[derive(Clone, Debug, Parser)]
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

    /// Bundler public key as string
    #[clap(
        long,
        env = "BUNDLER_PUBLIC",
        conflicts_with = "bundler-key",
        required_unless_present = "bundler-key"
    )]
    bundler_public: Option<String>,

    /// Path to JWK file holding bundler public key
    #[clap(
        long,
        env = "BUNDLER_KEY",
        conflicts_with = "bundler-public",
        required_unless_present = "bundler-public"
    )]
    bundler_key: Option<String>,

    /// Path to JWK file holding validator private key
    #[clap(long, env = "VALIDATOR_KEY")]
    validator_key: String,
}

#[derive(Clone)]
pub struct AppContext {
    key_manager: Arc<InMemoryKeyManager>,
    db_conn_pool: r2d2::Pool<ConnectionManager<SqliteConnection>>,
    redis_connection_url: String,
    listen: SocketAddr,
    validator_state: SharedValidatorState,
}

impl InMemoryKeyManagerConfig for (JsonWebKey, JsonWebKey) {
    fn bundler_jwk(&self) -> &JsonWebKey {
        &self.0
    }

    fn validator_jwk(&self) -> &JsonWebKey {
        &self.1
    }
}

impl AppContext {
    fn new(config: &AppConfig) -> Self {
        let bundler_jwk = if let Some(key_file_path) = &config.bundler_key {
            let file = fs::read_to_string(key_file_path).unwrap();
            file.parse().unwrap()
        } else {
            let n = config.bundler_public.as_ref().unwrap();
            JsonWebKey::new(Key::RSA {
                public: RsaPublic {
                    e: PublicExponent,
                    n: n.as_bytes().into(),
                },
                private: None,
            })
        };

        let validator_jwk: JsonWebKey = {
            let file = fs::read_to_string(&config.validator_key).unwrap();
            file.parse().unwrap()
        };

        let key_manager = InMemoryKeyManager::new(&(bundler_jwk, validator_jwk));
        let state = generate_state();

        let connection_mgr = ConnectionManager::<SqliteConnection>::new(&config.database_url);
        let pool = r2d2::Pool::builder()
            .build(connection_mgr)
            .expect("Failed to create SQLite connection pool.");

        if &config.database_url == ":memory:" {
            embedded_migrations::run(&pool.get().unwrap()).unwrap();
        }

        Self {
            key_manager: Arc::new(key_manager),
            db_conn_pool: pool,
            redis_connection_url: config.redis_connection_url.clone(),
            listen: config.listen,
            validator_state: state,
        }
    }
}

impl queries::RequestContext for AppContext {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
    }
}

impl RuntimeContext for AppContext {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
    }

    fn redis_connection_url(&self) -> &str {
        &self.redis_connection_url
    }

    fn bind_address(&self) -> &SocketAddr {
        &self.listen
    }
}

impl server::routes::sign::Config<Arc<InMemoryKeyManager>> for AppContext {
    fn bundler_address(&self) -> &str {
        self.key_manager.bundler_address()
    }

    fn validator_address(&self) -> &str {
        self.key_manager.validator_address()
    }

    fn current_epoch(&self) -> i64 {
        0
    }

    fn current_block(&self) -> u128 {
        0
    }

    fn key_manager(&self) -> &Arc<InMemoryKeyManager> {
        &self.key_manager
    }
}

impl ValidatorStateAccess for AppContext {
    fn get_validator_state(&self) -> &SharedValidatorState {
        &self.validator_state
    }
}

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().ok();

    let config = AppConfig::parse();
    let ctx = AppContext::new(&config);

    if !config.no_cron {
        paris::info!("Running with cron");
        tokio::task::spawn_local(run_crons(ctx.clone()));
    };

    if !config.no_server {
        paris::info!("Running with server");
        run_server(ctx.clone()).await.unwrap()
    };
}

#[cfg(test)]
pub mod test_utils {
    use std::sync::Arc;

    use crate::{
        embedded_migrations, key_manager::InMemoryKeyManager, state::generate_state, AppContext,
    };
    use diesel::{
        r2d2::{self, ConnectionManager},
        SqliteConnection,
    };

    pub fn test_context(key_manager: InMemoryKeyManager) -> AppContext {
        let connection_mgr = ConnectionManager::<SqliteConnection>::new(":memory:");
        let db_conn_pool = r2d2::Pool::builder()
            .build(connection_mgr)
            .expect("Failed to create SQLite connection pool.");

        {
            let conn = db_conn_pool.get().unwrap();
            embedded_migrations::run(&conn).unwrap();
        }

        let state = generate_state();

        AppContext {
            key_manager: Arc::new(key_manager),
            db_conn_pool,
            redis_connection_url: "".to_string(),
            listen: "127.0.0.1:10000".parse().unwrap(),
            validator_state: state,
        }
    }
}
