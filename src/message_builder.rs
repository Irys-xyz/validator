use std::fs;

use bundlr_sdk::{
    deep_hash::{DeepHashChunk, ONE_AS_BUFFER},
    deep_hash_sync::deep_hash_sync,
};
use clap::Parser;
use consts::BUNDLR_AS_BUFFER;
use data_encoding::BASE64URL_NOPAD;
use jsonwebkey::JsonWebKey;
use openssl::{hash::MessageDigest, rsa::Padding, sign};

mod consts;
mod key_manager;

#[derive(Clone, Debug, Parser)]
struct Args {
    /// Path to JWK file containing Arweaver wallet
    #[clap(short = 'w', long)]
    wallet: String,

    /// Bundlr transaction ID
    #[clap(short = 't', long)]
    tx: String,

    #[clap(short = 'b', long)]
    promised_block: u128,

    #[clap(short = 'v', long, min_values = 1, required = true)]
    validators: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let (private_key, _, _) = {
        let wallet = fs::read_to_string(&args.wallet).unwrap();
        let jwk: JsonWebKey = wallet.parse().unwrap();
        key_manager::split_jwk(&jwk)
    };

    let block = args.promised_block.to_string().as_bytes().to_vec();
    let tx_id = args.tx.as_bytes().to_vec();

    let signature_data = deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(BUNDLR_AS_BUFFER.into()),
        DeepHashChunk::Chunk(ONE_AS_BUFFER.into()),
        DeepHashChunk::Chunk(tx_id.into()),
        DeepHashChunk::Chunk(block.into()),
    ]))
    .unwrap();

    let (buf, len) = {
        let mut signer = sign::Signer::new(MessageDigest::sha256(), &private_key).unwrap();
        signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
        signer.update(&signature_data).unwrap();
        let mut buf = vec![0; 512];
        let len = signer.sign(&mut buf).unwrap();
        (buf, len)
    };

    let sig = BASE64URL_NOPAD.encode(&buf[0..len]);

    println!(
        r#"{{"id":"{}","signature":"{}","block":{},"validators":{}}}"#,
        &args.tx,
        sig,
        &args.promised_block,
        serde_json::to_string(&args.validators).unwrap()
    );
}
