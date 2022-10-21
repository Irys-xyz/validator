use actix_web::{web::Data, HttpResponse};
use serde::Serialize;

use crate::server::routes::sign::Config;
use crate::{key_manager, server::error::ValidatorServerError};

#[derive(Serialize)]
struct IndexBody<'a> {
    version: &'static str,
    address: &'a str,
    bundler_address: &'a str,
    block_height: u128,
    epoch: u128,
}

pub async fn index<Context, KeyManager>(
    ctx: Data<Context>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Context: self::Config<KeyManager>,
    KeyManager: key_manager::KeyManager,
{
    let body = IndexBody {
        version: env!("CARGO_PKG_VERSION"),
        address: &ctx.validator_address(),
        bundler_address: ctx.bundler_address(),
        block_height: ctx.current_block(),
        epoch: ctx.current_epoch(),
    };

    Ok(HttpResponse::Ok().json(body))
}
