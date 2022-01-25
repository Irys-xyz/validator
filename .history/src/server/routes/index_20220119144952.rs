use actix_web::HttpResponse;

struct IndexBody {
    
}

pub async fn index() -> actix_web::Result<HttpResponse> {
    let body = 
    Ok(HttpResponse::Ok().finish())
}