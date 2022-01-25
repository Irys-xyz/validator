use actix_web::{HttpResponse, web::Data};
use diesel_async::RunQueryDsl;

use crate::{server::error::ValidatorServerError, database::{schema::transactions::dsl::*, models::Transaction}, types::DbPool};
use diesel::prelude::*;

pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
        let conn = db.get().unwrap();
        let res = transactions
            .filter(id.eq("id"))
            .first::<Transaction>(&conn);

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}