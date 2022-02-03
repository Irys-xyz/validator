use diesel::Queryable;
use serde::Serialize;
use super::schema::transactions;
use super::schema::bundle;

#[derive(Serialize, Queryable)]
pub struct Bundle {
    pub id: String,
    pub owner_address: Option<String>,
    pub block_height: i64
}

#[derive (Insertable, Clone)]
#[table_name = "bundle"]
pub struct NewBundle {
    pub id: String,
    pub owner_address: Option<String>,
    pub block_height: i64
}

#[derive(Serialize, Queryable)]
pub struct Transaction {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: bool,
    pub bundle_id: Option<String>
}

#[derive(Insertable, Clone)]
#[table_name = "transactions"]
pub struct NewTransaction {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: bool,
    pub bundle_id: Option<String>
}