use std::{net::SocketAddr, sync::Arc};

use diesel::{
    r2d2::{self, ConnectionManager, PooledConnection},
    SqliteConnection,
};
use jsonwebkey::JsonWebKey;

use crate::{
    cron::arweave::ArweaveContext,
    database::queries,
    key_manager::{InMemoryKeyManager, InMemoryKeyManagerConfig, KeyManager},
    server::{self, RuntimeContext},
    state::{SharedValidatorState, ValidatorStateAccess}, http::reqwest::ReqwestClient,
};

struct Keys(JsonWebKey, JsonWebKey);

impl InMemoryKeyManagerConfig for Keys {
    fn bundler_jwk(&self) -> &JsonWebKey {
        &self.0
    }

    fn validator_jwk(&self) -> &JsonWebKey {
        &self.1
    }
}

#[derive(Clone)]
pub struct AppContext {
    key_manager: Arc<InMemoryKeyManager>,
    db_conn_pool: r2d2::Pool<ConnectionManager<SqliteConnection>>,
    listen: SocketAddr,
    validator_state: SharedValidatorState,
    http_client: reqwest::Client,
}

impl AppContext {
    pub fn new(
        key_manager: InMemoryKeyManager,
        db_conn_pool: r2d2::Pool<ConnectionManager<SqliteConnection>>,
        listen: SocketAddr,
        validator_state: SharedValidatorState,
        http_client: reqwest::Client,
    ) -> Self {
        Self {
            key_manager: Arc::new(key_manager),
            db_conn_pool,
            listen,
            validator_state,
            http_client,
        }
    }
}

impl queries::QueryContext for AppContext {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
    }

    fn current_epoch(&self) -> i64 {
        0
    }
}

impl RuntimeContext for AppContext {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
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

impl ArweaveContext<ReqwestClient> for AppContext {
    fn get_client(&self) -> ReqwestClient {
        self.http_client.clone()
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::sync::Arc;

    use super::AppContext;
    use crate::{key_manager::InMemoryKeyManager, state::generate_state};
    use diesel::{
        r2d2::{self, ConnectionManager},
        SqliteConnection,
    };

    embed_migrations!();

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
            listen: "127.0.0.1:10000".parse().unwrap(),
            validator_state: state,
            http_client: reqwest::Client::new(),
        }
    }
}
