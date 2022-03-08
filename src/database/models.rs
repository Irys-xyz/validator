use super::schema::bundle;
use super::schema::transactions;
use diesel::{Insertable, Queryable};
use serde::Serialize;

#[derive(Serialize, Queryable)]
pub struct Bundle {
    pub id: String,
    pub owner_address: Option<String>,
    pub block_height: i64,
}

#[derive(Insertable, Clone)]
#[table_name = "bundle"]
pub struct NewBundle {
    pub id: String,
    pub owner_address: Option<String>,
    pub block_height: i64,
}

#[derive(Serialize, Queryable)]
pub struct Transaction {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: i64,
    pub bundle_id: Option<String>,
    pub sent_to_leader: i64,
}

#[derive(Insertable, Clone, AsChangeset)]
#[table_name = "transactions"]
pub struct NewTransaction {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: i64,
    pub bundle_id: Option<String>,
    pub sent_to_leader: i64,
}
