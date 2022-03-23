use crate::database::queries;
use crate::http;
use crate::state::ValidatorRole;
use std::sync::Arc;

use super::arweave;
use super::bundle::{get_bundler, validate_bundler};
use super::error::ValidatorCronError;

pub async fn validate<Context, HttpClient>(ctx: Arc<Context>) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext + arweave::ArweaveContext<HttpClient>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    let bundler = get_bundler().await?;

    match ctx.get_validator_state().role() {
        ValidatorRole::Cosigner => validate_bundler(&*ctx, bundler).await?,
        ValidatorRole::Idle => (),
    }

    Ok(())
}

pub async fn validate_transactions<Context>(_: Arc<Context>) -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    super::bundle::validate_transactions(bundler).await?;

    Ok(())
}
