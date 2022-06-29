use sqlx::{Sqlite, Pool};


pub type DbPool = Pool<Sqlite>;

pub struct Validator {
    pub address: String,
    pub url: String
}