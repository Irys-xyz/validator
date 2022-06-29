use diesel::PgConnection;
use diesel_async::{deadpool::{Pool, ConnectionManager, ManagedAsyncConnection}, AsyncPgConnection};


pub type DbPool= Pool<ConnectionManager<PgConnection> + ManagedAsyncConnection>;

pub struct Validator {
    pub address: String,
    pub url: String
}