pub type DbPool = Pool<sqlite>;

pub struct Validator {
    pub address: String,
    pub url: String
}