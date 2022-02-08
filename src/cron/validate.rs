use super::bundle::{get_bundler, validate_bundler};
use super::error::ValidatorCronError;

pub async fn validate() -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    validate_bundler(bundler).await?;

    Ok(())
}

pub async fn validate_transactions() -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    super::bundle::validate_transactions(bundler).await?;

    Ok(())
}
