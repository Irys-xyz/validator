use actix_web::{HttpResponse, web::Data};

use crate::{server::error::ValidatorServerError, database::CassandraCtx};

type X = (String,);

struct Y {}

pub async fn get_tx(db: Data<CassandraCtx>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let CassandraCtx { session } = db.into();

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}