use actix_web::{HttpResponse, web::Data};

pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let mut conn = db.get()
        .await
        .unwrap();

    let res = transactions::dsl::transactions
        .filter(transactions::id.eq(""))
        .select(transactions::id)
        .first::<String>(&mut conn)
        .await;

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}