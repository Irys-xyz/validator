use diesel_async::{deadpool::{Pool, ConnectionManager, ManagedAsyncConnection}, AsyncPgConnection};


pub type DbPool = Pool<ConnectionManager<AsyncPgConnection>::AsyncPgConnection>;

pub struct Validator {
    pub address: String,
    pub url: String
}