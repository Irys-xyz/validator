use crate::state::SharedValidatorState;

use super::bundle::{get_bundler, validate_bundler};
use super::error::ValidatorCronError;

pub async fn validate(state: SharedValidatorState) -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    validate_bundler(bundler).await?;

    Ok(())
}

pub async fn validate_transactions(state: SharedValidatorState) -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    super::bundle::validate_transactions(bundler).await?;

    Ok(())
}
