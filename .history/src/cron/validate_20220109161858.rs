use super::{bundle::{validate_bundler, get_bundler}, error::ValidatorCronError};

pub async fn validate() -> () {
    let bundler = get_bundler().await.expect("");

    validate_bundler(bundler).await.expect()
}