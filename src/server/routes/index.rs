use actix_web::HttpResponse;

pub async fn index() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().finish())
}