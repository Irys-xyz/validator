use actix_web::{web::Data, HttpResponse};

use crate::{
    database::{models::Transaction, schema::transactions::dsl::*},
    server::{error::ValidatorServerError, RuntimeContext},
};
use diesel::prelude::*;

pub async fn get_tx<Context>(
    ctx: Data<Context>,
    path: (String,),
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Context: RuntimeContext,
{
    let conn = ctx.get_db_connection();
    let res = actix_rt::task::spawn_blocking(move || {
        transactions
            .filter(id.eq(path.0))
            .first::<Transaction>(&conn)
    })
    .await?;

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
