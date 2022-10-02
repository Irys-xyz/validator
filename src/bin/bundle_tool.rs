use std::{fmt, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};

use env_logger::Env;
use thiserror::Error;
use tokio::{
    fs::{DirBuilder, File},
    io::AsyncWriteExt,
};
use validator::bundlr::bundle::{
    extract_transaction_details, get_bundled_transactions, read_transaction_data, TransactionId,
};

#[derive(Debug, Error)]
struct RangeParseError;

impl fmt::Display for RangeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RangeParseError")
    }
}

struct Range {
    start: Option<usize>,
    end: Option<usize>,
}

impl FromStr for Range {
    type Err = RangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = if s.starts_with("(") { &s[1..] } else { &s[..] };
        let s = if s.ends_with(")") {
            &s[..s.len() - 1]
        } else {
            &s[..]
        };
        match s.split_once("..") {
            Some((start, end)) => {
                let start = if start.is_empty() {
                    None
                } else {
                    Some(start.parse::<usize>().map_err(|_| RangeParseError)?)
                };
                let end = if end.is_empty() {
                    None
                } else {
                    Some(end.parse::<usize>().map_err(|_| RangeParseError)?)
                };
                Ok(Self { start, end })
            }
            None => Err(RangeParseError),
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// List transactions in the bundle
    ListTransactions {
        /// Path to bundle
        #[clap(short = 'b', long)]
        bundle: PathBuf,

        /// Range of indexes that should be listed
        /// Syntax is "(start..end)" or just "start..end". When start is omitted,
        /// range starts from zero. When end is omitted, range ends to the last item.
        #[clap(short = 'r', long)]
        range: Option<Range>,
    },
    /// Extract bundled transaction from the bundle
    ExtractTransaction {
        /// Path to bundle
        #[clap(short = 'b', long)]
        bundle: PathBuf,

        /// Transaction ID
        #[clap(short = 't', long)]
        tx: TransactionId,

        /// Where to store extracted transaction data
        #[clap(long, env = "TX_CACHE", default_value = "./tx-cache/")]
        tx_cache: PathBuf,
    },
}

impl Command {
    async fn execute(self) {
        match self {
            Command::ListTransactions { bundle, range } => {
                let mut bundle_file = File::open(bundle)
                    .await
                    .expect("Failed to open bundle file");

                let transactions = get_bundled_transactions(&mut bundle_file)
                    .await
                    .expect("Failed to extract list of bundled transactions");

                let transactions = if let Some(range) = range {
                    match (range.start, range.end) {
                        (None, None) => &transactions[..],
                        (None, Some(end)) => &transactions[..end],
                        (Some(start), None) => &transactions[start..],
                        (Some(start), Some(end)) => &transactions[start..end],
                    }
                } else {
                    &transactions[..]
                };

                println!(
                    "{}",
                    serde_json::to_string(transactions)
                        .expect("Failed to deserialize transaction data")
                );
            }
            Command::ExtractTransaction {
                bundle,
                tx,
                tx_cache,
            } => {
                let mut bundle_file = File::open(bundle)
                    .await
                    .expect("Failed to open bundle file");

                let transactions = get_bundled_transactions(&mut bundle_file)
                    .await
                    .expect("Failed to extract list of bundled transactions");

                let tx_offset = match transactions.iter().find(|item| &item.id == &tx) {
                    Some(tx) => tx,
                    None => panic!("Requested transaction is not contained in the bundle"),
                };

                let tx = extract_transaction_details(&mut bundle_file, tx_offset)
                    .await
                    .expect("Failed to extract bundled transaction");

                if !tx_cache
                    .try_exists()
                    .expect("Failed to check if tx cache folder exists")
                {
                    DirBuilder::new()
                        .recursive(true)
                        .create(tx_cache.clone())
                        .await
                        .expect("Failed to create tx cache dir");
                }

                let mut tx_metadata_file_path = tx_cache.clone();
                tx_metadata_file_path.push(format!("{}.json", tx.id));

                let mut tx_metadata_file = File::create(tx_metadata_file_path.clone())
                    .await
                    .expect("Failed to open metadata file for writing");

                let json = serde_json::to_string(&tx).expect("Failed to serialize tx metadata");
                tx_metadata_file
                    .write(json.as_bytes())
                    .await
                    .expect("Failed to write metadata file");

                if let Some(ref data_offset) = tx.data_offset {
                    let mut tx_data_file_path = tx_cache;
                    tx_data_file_path.push(format!("{}.data", tx.id));

                    let mut tx_data_file = File::create(tx_data_file_path.clone())
                        .await
                        .expect("Failed to open data file for writing");

                    tx_data_file
                        .set_len(data_offset.size as u64)
                        .await
                        .expect("Failed to reserve space for data file");

                    read_transaction_data(&mut bundle_file, &mut tx_data_file, &tx)
                        .await
                        .expect("Failed to copy transaction data from bundle");
                }
            }
        }
    }
}

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();

    args.command.execute().await;
}
