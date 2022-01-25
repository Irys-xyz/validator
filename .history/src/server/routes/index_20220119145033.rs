use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
struct IndexBody {
    version: 
}

pub async fn index() -> actix_web::Result<HttpResponse> {
    let body = IndexBody {

    };

    Ok(HttpResponse::Ok().json(body))
}