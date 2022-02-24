use derive_more::{Display, Error};
use std::convert::From;

#[derive(Debug, Display, Error, Clone, PartialEq)]
pub enum ValidatorCronError {
    TxNotFound,
    AddressNotFound,
    TxsFromAddressNotFound,
    BundleNotInsertedInDB,
}

#[derive(Debug, Display, Error, Clone)]
pub enum TxsError {
    TxNotFound,
}

impl From<anyhow::Error> for ValidatorCronError {
    fn from(_err: anyhow::Error) -> ValidatorCronError {
        ValidatorCronError::AddressNotFound
    }
}
