use diesel_async::deadpool::{Pool, ConnectionManager};


pub type DbPool = Pool<ConnectionManager<AsyncPgConnection>>;

pub struct Validator {
    pub address: String,
    pub url: String
}