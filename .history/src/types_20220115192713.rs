use diesel_async::deadpool::Pool;


pub type DbPool = Pool<Connec<AsyncPgConnection>>;

pub struct Validator {
    pub address: String,
    pub url: String
}