use derive_more::{Display, Error};
use std::convert::From;

#[derive(Debug, Display, Error, Clone)]
pub enum ArweaveError {
    TxsNotFound,
    TagNotFound,
    UnknownErr
}

impl From<anyhow::Error> for ArweaveError {
    fn from(err: anyhow::Error) -> ArweaveError {
        ArweaveError::UnknownErr
    }
}