use actix_web::{web::Data, HttpResponse};
use diesel::dsl::count;
use diesel::{QueryDsl, RunQueryDsl};
use serde::Serialize;

use crate::database::schema::transactions::{self, id};
use crate::server::routes::sign::Config;
use crate::server::RuntimeContext;
use crate::{key_manager, server::error::ValidatorServerError};

#[derive(Serialize)]
struct StatusBody {
    total_txs: i64,
    epoch: u128,
    next_epoch: u128,
    previous_epoch: u128,
}

pub async fn status<Context, KeyManager>(
    ctx: Data<Context>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Context: self::Config<KeyManager> + RuntimeContext,
    KeyManager: key_manager::KeyManager,
{
    let conn = ctx.get_db_connection();
    let total_txs = transactions::table
        .select(count(id))
        .first(&conn)
        .unwrap_or(0 as i64);
    let current_epoch = ctx.current_epoch();

    let body = StatusBody {
        total_txs,
        epoch: current_epoch,
        next_epoch: current_epoch + 1,
        previous_epoch: current_epoch - 1,
    };

    Ok(HttpResponse::Ok().json(body))
}
