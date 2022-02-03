use super::{bundle::{validate_bundler, get_bundler}, error::ValidatorCronError, slasher::check_for_slash};

pub async fn validate() -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    validate_bundler(bundler).await?;

    Ok(())
} 