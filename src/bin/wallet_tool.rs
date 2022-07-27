use std::{fs, io::stdin};

use clap::{Parser, Subcommand};
use jsonwebkey::{JsonWebKey, Key, PublicExponent, RsaPrivate, RsaPublic};
use openssl::rsa::Rsa;

use validator::key_manager;

#[derive(Subcommand)]
enum Command {
    /// Create new wallet
    Create,
    /// Show Arweaver address for a wallet
    ShowAddress {
        /// Path to Arweave wallet file
        ///
        /// Provide path to Arweave wallet file or when not provided
        /// this application tries to read wallet data from stdin.
        #[clap(short = 'w', long)]
        wallet: Option<String>,
    },
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Create => {
            let rsa = Rsa::generate(4096)
                .expect("Failed to generate enough random data for the private key");

            let jwk = JsonWebKey::new(Key::RSA {
                public: RsaPublic {
                    e: PublicExponent,
                    n: rsa.n().to_vec().into(),
                },
                private: Some(RsaPrivate {
                    d: rsa.d().to_vec().into(),
                    p: rsa.p().map(|v| v.to_vec().into()),
                    q: rsa.q().map(|v| v.to_vec().into()),
                    dp: rsa.dmp1().map(|v| v.to_vec().into()),
                    dq: rsa.dmq1().map(|v| v.to_vec().into()),
                    qi: rsa.iqmp().map(|v| v.to_vec().into()),
                }),
            });

            println!("{}", jwk);
        }
        Command::ShowAddress { ref wallet } => {
            let (_, _, address) = {
                let wallet = if let Some(wallet) = wallet {
                    fs::read_to_string(wallet).expect("Failed to find wallet file")
                } else {
                    let res = stdin().lines().fold(String::new(), |mut acc, line| {
                        acc.push_str(&line.expect("Failed to read a line"));
                        acc
                    });
                    res
                };
                let jwk: JsonWebKey = wallet.parse().expect("Failed to parse wallet file");
                key_manager::split_jwk(&jwk)
            };

            println!(r#"{{"address":"{}"}}"#, address);
        }
    }
}
