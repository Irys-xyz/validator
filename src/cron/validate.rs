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
        + http::ClientAccess<HttpClient>
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

pub async fn validate_transactions<Context, HttpClient>(ctx: &Context) -> Result<(), CronJobError>
where
    Context: context::BundlerAccess + http::ClientAccess<HttpClient>,
    HttpClient: http::Client<Request = reqwest::Request, Response = reqwest::Response>,
{
    let http_client = ctx.get_http_client();
    super::bundle::validate_transactions(http_client, ctx.bundler())
        .await
        .map_err(CronJobError::ValidatorError)?;

    Ok(())
}
