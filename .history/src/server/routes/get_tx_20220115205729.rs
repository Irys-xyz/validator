use actix_web::{HttpResponse, web::Data};

use crate::{server::error::ValidatorServerError, types::DbPool};


pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let mut conn = db.acquire()
        .await
        .unwrap();

    let res = actix_rt::task::spawn_blocking(move || {
        let conn = db.get().unwrap();
        transactions
            .filter(id.eq("id"))
            .first::<Transaction>(&conn)
    })
        .await?;

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}