use crate::database::queries;
use crate::state::ValidatorRole;
use crate::{context, http, key_manager};

use super::bundle::validate_bundler;
use super::{arweave, CronJobError};

pub async fn validate<Context, HttpClient, KeyManager>(ctx: &Context) -> Result<(), CronJobError>
where
    Context: queries::QueryContext
        + arweave::ArweaveContext<HttpClient>
        + context::ArweaveAccess
        + context::BundlerAccess
        + key_manager::KeyManagerAccess<KeyManager>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
    KeyManager: key_manager::KeyManager,
{
    match ctx.get_validator_state().role() {
        ValidatorRole::Cosigner => validate_bundler(&*ctx)
            .await
            .map_err(CronJobError::ValidatorError)?,
        ValidatorRole::Idle => (),
    }

    Ok(())
}

pub async fn validate_transactions<Context>(ctx: &Context) -> Result<(), CronJobError>
where
    Context: context::BundlerAccess,
{
    super::bundle::validate_transactions(ctx.bundler())
        .await
        .map_err(CronJobError::ValidatorError)?;

    Ok(())
}
