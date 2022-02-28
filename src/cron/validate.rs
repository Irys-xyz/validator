use crate::state::{SharedValidatorState, ValidatorState};
use std::sync::atomic::Ordering;

use super::bundle::{get_bundler, validate_bundler};
use super::error::ValidatorCronError;

pub async fn validate(state: SharedValidatorState) -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    let s = state.load(Ordering::SeqCst);
    match s {
        s if s == ValidatorState::Cosigner => validate_bundler(bundler).await?,
        s if s == ValidatorState::Idle => (),
        s if s == ValidatorState::Leader => (),
        _ => panic!("Unknown validator state: {:?}", s),
    }

    Ok(())
}

pub async fn validate_transactions(_state: SharedValidatorState) -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    super::bundle::validate_transactions(bundler).await?;

    Ok(())
}
