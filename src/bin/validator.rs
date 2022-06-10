#[macro_use]
extern crate diesel_migrations;

use clap::Parser;
use data_encoding::BASE64URL_NOPAD;
use diesel::{
    r2d2::{self, ConnectionManager},
    sqlite::SqliteConnection,
};
use diesel_migrations::embed_migrations;
use env_logger::Env;
use jsonwebkey::{JsonWebKey, Key, PublicExponent, RsaPublic};
use std::{fs, net::SocketAddr, str::FromStr};
use url::Url;

use validator::{
    bundler::BundlerConfig,
    database::models::Bundle,
    http::reqwest::ReqwestClient,
    key_manager::{InMemoryKeyManager, InMemoryKeyManagerConfig},
};
use validator::{context::AppContext, state::generate_state};
use validator::{cron::run_crons, server::run_server};

embed_migrations!();

#[derive(Clone, Debug, Parser)]
struct CliOpts {
    /// Do not start cron jobs
    #[clap(long)]
    no_cron: bool,

    /// Do not start app in server mode
    #[clap(long)]
    no_server: bool,

    /// Database connection URL
    #[clap(long, env, default_value = "validator.db")]
    database_url: String,

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

    /// URL for the bundler connection
    #[clap(long, env = "BUNDLER_URL")]
    bundler_url: Url,

    /// Path to JWK file holding validator private key
    #[clap(long, env = "VALIDATOR_KEY")]
    validator_key: String,

    #[clap(long, env = "ARWEAVE_URL")]
    arweave_url: Option<Url>,
}

fn merge_configs(config: CliOpts, bundler_config: BundlerConfig) -> CliOpts {
    let arweave_url = match config.arweave_url {
        Some(u) => Some(u),
        None => {
            let url_string = format!("https://{}", bundler_config.gateway);
            let url = url::Url::from_str(&url_string).unwrap();
            Some(url)
        }
    };

    CliOpts {
        arweave_url,
        ..config
    }
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

impl From<&CliOpts> for AppContext {
    fn from(config: &CliOpts) -> Self {
        let bundler_jwk = if let Some(key_file_path) = &config.bundler_key {
            let file = fs::read_to_string(key_file_path).unwrap();
            file.parse().unwrap()
        } else {
            let n = config.bundler_public.as_ref().unwrap();
            JsonWebKey::new(Key::RSA {
                public: RsaPublic {
                    e: PublicExponent,
                    n: BASE64URL_NOPAD
                        .decode(n.as_bytes().into())
                        .expect("Failed to decode bundler's public key")
                        .into(),
                },
                private: None,
            })
        };

        let validator_jwk: JsonWebKey = {
            let file = fs::read_to_string(&config.validator_key).unwrap();
            file.parse().unwrap()
        };

        let key_manager = InMemoryKeyManager::new(&Keys(bundler_jwk, validator_jwk));
        let state = generate_state();

        let connection_mgr = ConnectionManager::<SqliteConnection>::new(&config.database_url);
        let pool = r2d2::Pool::builder()
            .build(connection_mgr)
            .expect("Failed to create SQLite connection pool.");

        if &config.database_url == ":memory:" {
            embedded_migrations::run(&pool.get().unwrap()).unwrap();
        }

        Self::new(
            key_manager,
            pool,
            config.listen,
            state,
            reqwest::Client::new(),
            config.arweave_url.as_ref(),
            &config.bundler_url,
        )
    }
}

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().ok();

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let http_client = ReqwestClient::new(reqwest::Client::new());
    let app_config = CliOpts::parse();
    let bundler_config = BundlerConfig::fetch_config(http_client, &app_config.bundler_url).await;
    let config = merge_configs(app_config, bundler_config);
    let ctx = AppContext::from(&config);

    if !config.no_cron {
        paris::info!("Running with cron");
        tokio::task::spawn_local(run_crons(ctx.clone()));
    };

    if !config.no_server {
        paris::info!("Running with server");
        run_server(ctx.clone()).await.unwrap()
    };
}
