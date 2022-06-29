use actix_web::{HttpResponse, web::Data};
use diesel_async::{deadpool::{Pool, ConnectionManager, ManagedAsyncConnection}, AsyncPgConnection};

use crate::{server::error::ValidatorServerError, database::{models::Transaction, schema}, types::DbPool};
use schema::transactions;

pub async fn get_tx(db: Data<Pool<ConnectionManager<Asyn>>>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let mut db = db.into_inner().as_ref();

    let res = transactions::table
        .filter(transactions::dsl::id.eq("id"))
        .first::<Transaction>(&mut db)
        .await
        .unwrap();

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}