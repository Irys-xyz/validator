use derive_more::{Display, Error};
use std::convert::From;

#[derive(Debug, Display, Error, Clone)]
pub enum ValidatorCronError {
    TxNotFound,
    AddressNotFound
}

impl From<anyhow::Error> for ValidatorCronError {
    fn from(err: anyhow::Error) -> ValidatorCronError {
        ValidatorCronError::AddressNotFound
    }
}