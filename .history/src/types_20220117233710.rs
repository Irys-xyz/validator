use sqlx::{Sqlite, Pool};


pub struct Validator {
    pub address: String,
    pub url: String
}