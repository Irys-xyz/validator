use derive_more::{Display, Error};
use std::convert::From;

#[derive(Debug, Display, Error, Clone)]
pub enum ArweaveError {
    TxsNotFound,
    MalformedQuery,
    InternalServerError,
    GatewayTimeout,
    UnknownErr,
}

impl From<anyhow::Error> for ArweaveError {
    fn from(_err: anyhow::Error) -> ArweaveError {
        ArweaveError::UnknownErr
    }
}
