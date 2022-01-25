use actix_web::{HttpResponse, web::Data};
use diesel_async::deadpool::Pool;

use crate::{server::error::ValidatorServerError, types::DbPool};


pub async fn get_tx(db: Data<Pool<>>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let mut conn = db.acquire()
        .await
        .unwrap();

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}