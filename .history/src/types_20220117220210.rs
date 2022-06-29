use diesel_async::AsyncPgConnection;

pub type DbPool = bool<AsyncPgConnection>;

pub struct Validator {
    pub address: String,
    pub url: String
}