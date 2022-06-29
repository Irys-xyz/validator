use diesel::Queryable;
use serde::Serialize;
use super::schema::transactions;

#[derive(Serialize, Queryable)]
pub struct Transaction {
    id: String,
    epoch: i64,
    block_promised: i64,
    block_actual: Option<i64>,
    signature: Vec<u8>,
    validated: bool
}

#[derive(Insertable, Clone)]
#[table_name = "transactions"]
pub struct NewTransaction<'a> {
    pub id: String,
    pub epoch: i64,
    pub block_promised: i64,
    pub block_actual: Option<i64>,
    pub signature: Vec<u8>,
    pub validated: bool
}