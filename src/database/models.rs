use super::schema::bundle;
use super::schema::transactions;
use diesel::{Insertable, Queryable};
use serde::Serialize;

#[derive(Serialize, Queryable)]
pub struct Bundle {
    pub id: Option<String>,
    pub owner_address: Option<String>,
    pub block_height: i32,
}

#[derive(Insertable, Clone)]
#[table_name = "bundle"]
pub struct NewBundle {
    pub id: Option<String>,
    pub owner_address: Option<String>,
    pub block_height: i32,
}

#[derive(Serialize, Queryable)]
pub struct Transaction {
    pub id: Option<String>,
    pub epoch: i32,
    pub block_promised: i32,
    pub block_actual: Option<i32>,
    pub signature: Vec<u8>,
    pub validated: i32,
    pub bundle_id: Option<String>,
    pub sent_to_leader: i32,
}

#[derive(Insertable, Clone, AsChangeset)]
#[table_name = "transactions"]
pub struct NewTransaction {
    pub id: Option<String>,
    pub epoch: i32,
    pub block_promised: i32,
    pub block_actual: Option<i32>,
    pub signature: Vec<u8>,
    pub validated: i32,
    pub bundle_id: Option<String>,
    pub sent_to_leader: i32,
}
