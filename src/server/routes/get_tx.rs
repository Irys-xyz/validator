use std::sync::{atomic::Ordering, Arc};

use actix_web::{web::Data, HttpResponse};

use crate::{
    database::{models::Transaction, schema::transactions::dsl::*},
    server::error::ValidatorServerError,
    state::ValidatorStateTrait,
    types::DbPool,
};
use diesel::prelude::*;

pub async fn get_tx<Config>(
    _ctx: Data<Config>,
    db: Data<DbPool>,
    path: (String,),
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Config: ValidatorStateTrait,
{
    let res = actix_rt::task::spawn_blocking(move || {
        let conn = db.get().unwrap();
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
