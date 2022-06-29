use actix_web::{HttpResponse, web::Data};
use diesel_async::*;
use diesel::prelude::ExpressionMethods;
use futures::future::join_all;

use crate::{server::error::ValidatorServerError, database::{models::Transaction, schema}, types::DbPool};
use schema::transactions;
pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let db = db.into_inner();
        let conn = db.get()
            .await
            .unwrap();
        let res = transactions::table
            .filter(transactions::dsl::id.eq("id"))
            .load::<Transaction>(&mut conn)
            .await
            .unwrap();

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}