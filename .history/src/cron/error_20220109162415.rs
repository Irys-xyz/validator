use derive_more::{Display, Error};

#[derive(Debug, Display, Error, Clone)]
pub enum ValidatorCronError {
    TxNotFound
}