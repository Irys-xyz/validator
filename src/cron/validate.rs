use std::sync::Arc;

use crate::database::queries;
use crate::state::ValidatorRole;
use crate::{context, http};

use super::arweave;
use super::bundle::validate_bundler;
use super::error::ValidatorCronError;

pub async fn validate<Context, HttpClient>(ctx: Arc<Context>) -> Result<(), ValidatorCronError>
where
    Context: queries::QueryContext + arweave::ArweaveContext<HttpClient> + context::BundlerAccess,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    match ctx.get_validator_state().role() {
        ValidatorRole::Cosigner => validate_bundler(&*ctx).await?,
        ValidatorRole::Idle => (),
    }

    Ok(())
}

pub async fn validate_transactions<Context>(ctx: Arc<Context>) -> Result<(), ValidatorCronError>
where
    Context: context::BundlerAccess,
{
    super::bundle::validate_transactions(ctx.bundler()).await?;

    Ok(())
}
