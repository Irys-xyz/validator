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
use data_encoding::BASE64URL_NOPAD;
use database::queries;
use diesel::{Connection, PgConnection};
use jsonwebkey::{JsonWebKey, Key, PublicExponent, RsaPublic};
use openssl::{
    pkey::{PKey, Private, Public},
    sha::Sha256,
};
use server::{run_server, RuntimeContext};
use state::generate_state;
use std::{fs, net::SocketAddr};

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
struct AppContext {
    bundler_key: PKey<Public>,
    validator_private_key: PKey<Private>,
    validator_public_key: PKey<Public>,
    bundler_address: String,
    validator_address: String,
    database_url: String,
    redis_connection_url: String,
    listen: SocketAddr,
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

        let (bundler_public, bundler_address) = {
            let jwk = bundler_jwk;
            let der = if jwk.key.is_private() {
                let pub_key = jwk.key.to_public().unwrap();
                pub_key.try_to_der().unwrap()
            } else {
                jwk.key.try_to_der().unwrap()
            };
            let pub_key = PKey::public_key_from_der(der.as_slice()).unwrap();
            let mut hasher = Sha256::new();
            hasher.update(&pub_key.rsa().unwrap().n().to_vec());
            let hash = hasher.finish();
            let address = BASE64URL_NOPAD.encode(&hash);
            (pub_key, address)
        };
        let (validator_private, validator_public, validator_address) = {
            let jwk = validator_jwk;
            let priv_key = {
                let der = jwk.key.try_to_der().unwrap();
                PKey::private_key_from_der(der.as_slice()).unwrap()
            };
            let pub_key = {
                let pub_key_part = jwk.key.to_public().unwrap();
                let der = pub_key_part.try_to_der().unwrap();
                PKey::public_key_from_der(der.as_slice()).unwrap()
            };
            let mut hasher = Sha256::new();
            hasher.update(&pub_key.rsa().unwrap().n().to_vec());
            let hash = hasher.finish();
            let address = BASE64URL_NOPAD.encode(&hash);
            (priv_key, pub_key, address)
        };

        Self {
            bundler_key: bundler_public,
            validator_private_key: validator_private,
            validator_public_key: validator_public,
            bundler_address,
            validator_address,
            database_url: config.database_url.clone(),
            redis_connection_url: config.redis_connection_url.clone(),
            listen: config.listen,
        }
    }
}

impl queries::RequestContext for AppContext {
    // FIXME: this should use connection pool
    fn get_db_connection(&self) -> PgConnection {
        PgConnection::establish(&self.database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", self.database_url))
    }
}

impl RuntimeContext for AppContext {
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

impl server::routes::sign::Config for AppContext {
    fn bundler_address(&self) -> &str {
        &self.bundler_address
    }

    fn bundler_public_key(&self) -> &openssl::pkey::PKey<openssl::pkey::Public> {
        &self.bundler_key
    }

    fn validator_address(&self) -> &str {
        &self.validator_address
    }

    fn validator_private_key(&self) -> &openssl::pkey::PKey<openssl::pkey::Private> {
        &self.validator_private_key
    }

    fn validator_public_key(&self) -> &openssl::pkey::PKey<Public> {
        &self.validator_public_key
    }
}

#[actix_web::main]
async fn main() -> () {
    dotenv::dotenv().ok();

    let config = AppConfig::parse();
    let state = generate_state();

    let ctx = AppContext::new(&config);

    if !config.no_cron {
        paris::info!("Running with cron");
        tokio::task::spawn_local(run_crons(ctx.clone(), state));
    };

    if !config.no_server {
        paris::info!("Running with server");
        run_server(ctx.clone()).await.unwrap()
    };
}
