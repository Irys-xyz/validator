use actix_web::HttpResponse;

#[derive()]
struct IndexBody {

}

pub async fn index() -> actix_web::Result<HttpResponse> {
    let body = 
    Ok(HttpResponse::Ok().finish())
}