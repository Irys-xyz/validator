use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub struct Validator {
    pub address: String,
    pub url: String,
}
