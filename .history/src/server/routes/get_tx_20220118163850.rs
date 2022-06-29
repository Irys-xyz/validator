use actix_web::{HttpResponse, web::Data};

use crate::{server::error::ValidatorServerError, database::CassandraCtx, types::DbPool};

type X = (String,);

struct Y {}

pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let conn = db.acquire()
        .await
        .unwrap();

    let res = sqlx::query_as!(Y, "");
    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}