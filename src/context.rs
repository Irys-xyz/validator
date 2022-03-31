use std::{net::SocketAddr, sync::Arc};

use diesel::{
    r2d2::{self, ConnectionManager, PooledConnection},
    SqliteConnection,
};
use jsonwebkey::JsonWebKey;

use crate::{
    cron::arweave::ArweaveContext,
    database::queries,
    http::reqwest::ReqwestClient,
    key_manager::{InMemoryKeyManager, InMemoryKeyManagerConfig, KeyManager},
    server::{self, RuntimeContext},
    state::{SharedValidatorState, ValidatorStateAccess},
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
pub struct AppContext<HttpClient = ReqwestClient> {
    key_manager: Arc<InMemoryKeyManager>,
    db_conn_pool: r2d2::Pool<ConnectionManager<SqliteConnection>>,
    listen: SocketAddr,
    validator_state: SharedValidatorState,
    http_client: HttpClient,
    arweave_uri: http::uri::Uri,
}

impl AppContext {
    pub fn new(
        key_manager: InMemoryKeyManager,
        db_conn_pool: r2d2::Pool<ConnectionManager<SqliteConnection>>,
        listen: SocketAddr,
        validator_state: SharedValidatorState,
        http_client: reqwest::Client,
        arweave_uri: http::uri::Uri,
    ) -> Self {
        Self {
            key_manager: Arc::new(key_manager),
            db_conn_pool,
            listen,
            validator_state,
            http_client: ReqwestClient::new(http_client),
            arweave_uri,
        }
    }
}

impl<HttpClient> queries::QueryContext for AppContext<HttpClient> {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
    }

    fn current_epoch(&self) -> i64 {
        0
    }
}

impl<HttpClient> RuntimeContext for AppContext<HttpClient> {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<SqliteConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
    }

    fn bind_address(&self) -> &SocketAddr {
        &self.listen
    }
}

impl<HttpClient> server::routes::sign::Config<Arc<InMemoryKeyManager>> for AppContext<HttpClient> {
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

impl<HttpClient> ValidatorStateAccess for AppContext<HttpClient> {
    fn get_validator_state(&self) -> &SharedValidatorState {
        &self.validator_state
    }
}

impl<HttpClient> ArweaveContext<HttpClient> for AppContext<HttpClient>
where
    HttpClient:
        crate::http::Client<Request = reqwest::Request, Response = reqwest::Response> + Clone,
{
    fn get_client(&self) -> HttpClient {
        self.http_client.clone()
    }

    fn get_arweave_uri(&self) -> &http::uri::Uri {
        &self.arweave_uri
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::{str::FromStr, sync::Arc};

    use super::AppContext;
    use crate::{
        http::reqwest::mock::MockHttpClient, key_manager::InMemoryKeyManager, state::generate_state,
    };
    use diesel::{
        r2d2::{self, ConnectionManager},
        SqliteConnection,
    };

    embed_migrations!();

    pub fn test_context(key_manager: InMemoryKeyManager) -> AppContext<MockHttpClient> {
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
            http_client: MockHttpClient::new(|_, _| false),
            arweave_uri: http::uri::Uri::from_str(&"http://example.com".to_string()).unwrap(),
        }
    }

    pub fn test_context_with_http_client<HttpClient>(
        key_manager: InMemoryKeyManager,
        http_client: HttpClient,
    ) -> AppContext<HttpClient> {
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
            http_client,
            arweave_uri: http::uri::Uri::from_str(&"http://example.com".to_string()).unwrap(),
        }
    }
}
