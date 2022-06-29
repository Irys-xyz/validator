use diesel_async::{deadpool::{Pool, ConnectionManager}, AsyncPgConnection};


pub type DbPool = Pool<AsyncPgConnection>>;

pub struct Validator {
    pub address: String,
    pub url: String
}