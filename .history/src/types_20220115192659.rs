use diesel::{r2d2::{Pool, ConnectionManager}, PgConnection};

pub type DbPool = Pool<Connec<PgConnection>>;

pub struct Validator {
    pub address: String,
    pub url: String
}