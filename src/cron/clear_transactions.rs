use crate::{database::{queries::{QueryContext, filter}}};

use super::CronJobError;
use crate::cron::ValidatorCronError;

pub async fn clear_old_transactions<Context>(ctx: &Context) -> Result<(), CronJobError>
where
  Context: QueryContext 
{
  let epoch = ctx.current_epoch();
  filter(ctx, epoch, 40).await
    .map(|amount| print!( "Deleted {} transactions from epoch {} to {}", amount, epoch - 40, epoch))
    .map_err(|err| CronJobError::ValidatorError(ValidatorCronError::from(err)))
}