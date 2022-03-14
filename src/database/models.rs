use std::hash::Hasher;
use std::io::Write;

use super::schema::bundle;
use super::schema::transactions;
use diesel::backend::Backend;
use diesel::deserialize;
use diesel::query_builder::AsChangeset;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Binary;
use diesel::types::FromSql;
use diesel::Expression;
use diesel::{Insertable, Queryable};
use serde::Serialize;

#[derive(Serialize, Queryable)]
pub struct Bundle {
    pub id: String,
    pub owner_address: String,
    pub block_height: i64,
}

#[derive(Insertable, Clone)]
#[table_name = "bundle"]
pub struct NewBundle {
    pub id: String,
    pub owner_address: String,
    pub block_height: i64,
}

#[derive(Serialize, Queryable)]
pub struct Transaction {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: bool,
    pub bundle_id: Option<String>,
    pub sent_to_leader: bool,
}

#[derive(Insertable, Clone, AsChangeset)]
#[table_name = "transactions"]
pub struct NewTransaction {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: bool,
    pub bundle_id: Option<String>,
    pub sent_to_leader: bool,
}
