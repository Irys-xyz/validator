use diesel_async::{deadpool::{Pool, ConnectionManager, ManagedAsyncConnection}, AsyncPgConnection};


pub type DbPool<T: ManagedAsyncConnection> = Pool<ConnectionManager<Manag>>;

pub struct Validator {
    pub address: String,
    pub url: String
}