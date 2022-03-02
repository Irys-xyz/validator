use crate::database::queries;
use crate::state::{SharedValidatorState, ValidatorState};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::bundle::{get_bundler, validate_bundler};
use super::error::ValidatorCronError;

pub async fn validate<Context>(ctx: Arc<Context>) -> Result<(), ValidatorCronError>
where
    Context: queries::RequestContext,
{
    let bundler = get_bundler().await?;

    let s = ctx.get_validator_state().load(Ordering::SeqCst);
    match s {
        s if s == ValidatorState::Cosigner => validate_bundler(&*ctx, bundler).await?,
        s if s == ValidatorState::Idle => (),
        s if s == ValidatorState::Leader => (),
        _ => panic!("Unknown validator state: {:?}", s),
    }

    Ok(())
}

pub async fn validate_transactions<Context>(_: Arc<Context>) -> Result<(), ValidatorCronError> {
    let bundler = get_bundler().await?;

    super::bundle::validate_transactions(bundler).await?;

    Ok(())
}
