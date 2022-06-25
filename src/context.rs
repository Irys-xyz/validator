use std::{net::SocketAddr, sync::Arc};

use diesel::{
    r2d2::{self, ConnectionManager, PooledConnection},
    PgConnection,
};
use jsonwebkey::JsonWebKey;
use url::Url;

use crate::{
    arweave::{Arweave, ArweaveContext},
    bundler::Bundler,
    contract_gateway::ContractGateway,
    database::queries,
    http::reqwest::ReqwestClient,
    key_manager::{InMemoryKeyManager, InMemoryKeyManagerConfig, KeyManager, KeyManagerAccess},
    server::{self, RuntimeContext},
    state::{SharedValidatorState, ValidatorStateAccess},
};

pub trait BundlerAccess {
    fn bundler(&self) -> &Bundler;
}

pub trait ArweaveAccess {
    fn arweave(&self) -> &Arweave;
}

pub trait ValidatorAddressAccess {
    fn get_validator_address(&self) -> &str;
}

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
    db_conn_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    listen: SocketAddr,
    validator_state: SharedValidatorState,
    http_client: HttpClient,
    arweave_client: Arweave,
    bundler_connection: Bundler,
    contract_gateway: ContractGateway,
}

impl AppContext {
    pub fn new(
        key_manager: InMemoryKeyManager,
        db_conn_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
        listen: SocketAddr,
        validator_state: SharedValidatorState,
        http_client: reqwest::Client,
        arweave_url: &Url,
        bundler_url: &Url,
        contract_gateway_url: &Url,
    ) -> Self {
        let bundler_connection = Bundler {
            address: key_manager.bundler_address().to_owned(),
            url: bundler_url.clone(),
        };

        let arweave_client = Arweave {
            url: arweave_url.clone(),
        };

        let contract_gateway = ContractGateway {
            url: contract_gateway_url.clone(),
        };

        Self {
            key_manager: Arc::new(key_manager),
            db_conn_pool,
            listen,
            validator_state,
            http_client: ReqwestClient::new(http_client),
            arweave_client,
            bundler_connection,
            contract_gateway,
        }
    }
}

impl<HttpClient> BundlerAccess for AppContext<HttpClient> {
    fn bundler(&self) -> &Bundler {
        &self.bundler_connection
    }
}

impl<HttpClient> ArweaveAccess for AppContext<HttpClient> {
    fn arweave(&self) -> &Arweave {
        &self.arweave_client
    }
}

impl<HttpClient> KeyManagerAccess<InMemoryKeyManager> for AppContext<HttpClient> {
    fn get_key_manager(&self) -> &InMemoryKeyManager {
        self.key_manager.as_ref()
    }
}

impl<HttpClient> crate::http::ClientAccess<HttpClient> for AppContext<HttpClient>
where
    HttpClient:
        crate::http::Client<Request = reqwest::Request, Response = reqwest::Response> + Clone,
{
    fn get_http_client(&self) -> &HttpClient {
        &self.http_client
    }
}

impl<HttpClient> crate::contract_gateway::ContractGatewayAccess for AppContext<HttpClient> {
    fn contract_gateway(&self) -> &ContractGateway {
        &self.contract_gateway
    }
}

impl<HttpClient> ArweaveContext<HttpClient> for AppContext<HttpClient>
where
    HttpClient:
        crate::http::Client<Request = reqwest::Request, Response = reqwest::Response> + Clone,
{
    fn get_client(&self) -> &HttpClient {
        &self.http_client
    }
}

impl<HttpClient> queries::QueryContext for AppContext<HttpClient> {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.db_conn_pool
            .get()
            .expect("Failed to get connection from database connection pool")
    }

    fn current_epoch(&self) -> u128 {
        self.validator_state.current_epoch()
    }
}

impl<HttpClient> RuntimeContext for AppContext<HttpClient> {
    fn get_db_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
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

    fn current_epoch(&self) -> u128 {
        self.validator_state.current_epoch()
    }

    fn current_block(&self) -> u128 {
        self.validator_state.current_block()
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

impl<HttpClient> ValidatorAddressAccess for AppContext<HttpClient> {
    fn get_validator_address(&self) -> &str {
        self.key_manager.validator_address()
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::{str::FromStr, sync::Arc};

    use super::AppContext;
    use crate::{
        arweave::Arweave,
        bundler::Bundler,
        contract_gateway::ContractGateway,
        http::reqwest::mock::MockHttpClient,
        key_manager::{InMemoryKeyManager, KeyManager},
        state::generate_state,
    };
    use diesel::{
        r2d2::{self, ConnectionManager},
        PgConnection,
    };
    use diesel_migrations::embed_migrations;
    use url::Url;

    embed_migrations!();

    pub fn test_context(key_manager: InMemoryKeyManager) -> AppContext<MockHttpClient> {
        let mgr =
            ConnectionManager::<PgConnection>::new("postgres://bundlr:bundlr@localhost/bundlr");
        let db_conn_pool = r2d2::Pool::builder()
            .build(mgr)
            .expect("could not build connection pool");

        let state = generate_state();

        let bundler_connection = Bundler {
            address: key_manager.bundler_address().to_owned(),
            url: Url::from_str("http://bundler.example.com").unwrap(),
        };

        let arweave_client = Arweave {
            url: Url::from_str("http://example.com").unwrap(),
        };

        let contract_gateway = ContractGateway {
            url: Url::from_str("http://localhost:3000").unwrap(),
        };

        AppContext {
            key_manager: Arc::new(key_manager),
            db_conn_pool,
            listen: "127.0.0.1:42069".parse().unwrap(),
            validator_state: state,
            http_client: MockHttpClient::new(|_, _| false),
            arweave_client,
            bundler_connection,
            contract_gateway,
        }
    }

    pub fn test_context_with_http_client<HttpClient>(
        key_manager: InMemoryKeyManager,
        http_client: HttpClient,
    ) -> AppContext<HttpClient> {
        let mgr =
            ConnectionManager::<PgConnection>::new("postgres://bundlr:bundlr@localhost/bundlr");
        let db_conn_pool = r2d2::Pool::builder()
            .build(mgr)
            .expect("could not build connection pool");

        let state = generate_state();

        let bundler_connection = Bundler {
            address: key_manager.bundler_address().to_owned(),
            url: Url::from_str("http://bundler.example.com").unwrap(),
        };

        let arweave_client = Arweave {
            url: Url::from_str("http://example.com").unwrap(),
        };

        let contract_gateway = ContractGateway {
            url: Url::from_str("http://localhost:3000").unwrap(),
        };

        AppContext {
            key_manager: Arc::new(key_manager),
            db_conn_pool,
            listen: "127.0.0.1:42069".parse().unwrap(),
            validator_state: state,
            http_client,
            arweave_client,
            bundler_connection,
            contract_gateway,
        }
    }
}
