use actix_web::{HttpResponse, web::Data};
use diesel_async::*;
use diesel::{prelude::ExpressionMethods, QueryDsl};
use futures::future::join_all;

use crate::{server::error::ValidatorServerError, database::{models::Transaction, schema}, types::DbPool};
use schema::transactions;
pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let mut db = db.into_inner().as_ref();
    let x = db.get().await.unwrap();

        // let res = transactions::table
        //     .filter(transactions::dsl::id.eq("id"))
        //     .load::<Transaction>(&mut db)
        //     .await
        //     .unwrap();

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}