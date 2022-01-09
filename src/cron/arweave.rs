use std::{collections::HashMap, str::FromStr};

use awc::Client;
use bigint::U256;
use futures::{select, stream::FuturesUnordered, TryStreamExt, StreamExt};
use serde::Deserialize;
use lazy_static::lazy_static;


pub async fn is_seeded(tx_id: String) -> bool {
    let peers = Vec::<String>::new();
    let client = Client::default();
    
    let f = FuturesUnordered::from_iter(peers.into_iter().map(|p| check_seeded_at(client.clone(), p, tx_id.as_str())))
        .take(5)
        .collect::<Vec<_>>()
        .await;

    return true;
}

#[derive(Deserialize)]
struct OffsetResponse {
    size: String,
    offset: String
}

#[derive(Deserialize)]
struct StatusResponse {
    confirmations: u128
}

async fn check_seeded_at(client: Client, peer: String, tx_id: &str) -> () {
    let offset_response: OffsetResponse = client.get(format!("http://{}/tx/{}/offset", peer, tx_id))
        .content_type("application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let OffsetResponse { size, offset } = offset_response;
    let size = U256::from_dec_str(&size).unwrap();
    let offset = U256::from_dec_str(&offset).unwrap();

    let mut response = client.get(format!("http://{}/data_sync_record/{}/1", peer, offset - size))
        .content_type("application/json")
        .send()
        .await
        .unwrap();

    if !response.status().is_success() {
        panic!("Not successful");
    };

    let body: HashMap<String, String> = response
        .json()
        .await
        .unwrap();


    let status_response: StatusResponse = client.get(format!("http://{}/tx/{}/status", peer, tx_id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    
}