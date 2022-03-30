use crate::{
    server::error::ValidatorServerError,
    state::{ValidatorRole, ValidatorStateAccess},
};
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use serde::{de, Deserialize, Deserializer};

/// Deserializer from string to u128
fn de_optional_u128<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u128>, D::Error> {
    let s: &str = de::Deserialize::deserialize(deserializer)?;
    s.parse().map(Some).map_err(de::Error::custom)
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Request {
    #[serde(deserialize_with = "de_optional_u128", default)]
    epoch: Option<u128>,
    #[serde(deserialize_with = "de_optional_u128", default)]
    block: Option<u128>,
    #[serde(default)]
    role: Option<ValidatorRole>,
}

pub async fn set_state<Context>(
    ctx: Data<Context>,
    req: Json<Request>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Context: ValidatorStateAccess,
{
    let state = ctx.get_validator_state();

    if let Some(epoch) = req.epoch {
        state.set_current_epoch(epoch)
    }

    if let Some(block) = req.block {
        state.set_current_block(block)
    }

    if let Some(role) = req.role {
        state.set_role(role)
    }

    Ok(HttpResponse::Ok().finish())
}

#[cfg(test)]
mod tests {
    use actix_web::{
        http::header::ContentType,
        middleware::Logger,
        test::{call_service, init_service, TestRequest},
        web::{self, Data},
        App,
    };

    use crate::{
        context::{test_utils::test_context, AppContext},
        http::reqwest::mock::MockHttpClient,
        server::routes::test::set_state,
        state::{ValidatorRole, ValidatorStateAccess},
    };

    use super::Request;

    #[test]
    fn deserialize_set_epoch_request() {
        let body = r#"{"epoch":"1"}"#;

        let req: Request = serde_json::from_str(body).unwrap();

        assert_eq!(
            req,
            Request {
                epoch: Some(1),
                block: None,
                role: None
            }
        );
    }

    #[test]
    fn deserialize_set_current_block_request() {
        let body = r#"{"block":"1"}"#;

        let req: Request = serde_json::from_str(body).unwrap();

        assert_eq!(
            req,
            Request {
                epoch: None,
                block: Some(1),
                role: None
            }
        );
    }

    #[test]
    fn deserialize_set_role_request() {
        let body = r#"{"role":"idle"}"#;

        let req: Request = serde_json::from_str(body).unwrap();

        assert_eq!(
            req,
            Request {
                epoch: None,
                block: None,
                role: Some(ValidatorRole::Idle)
            }
        );
    }

    #[test]
    fn deserialize_set_all_fields_request() {
        let body = r#"{"role":"cosigner","block":"1","epoch":"1"}"#;

        let req: Request = serde_json::from_str(body).unwrap();

        assert_eq!(
            req,
            Request {
                epoch: Some(1),
                block: Some(1),
                role: Some(ValidatorRole::Cosigner)
            }
        );
    }

    #[actix_web::test]
    async fn sending_request_to_set_role_yields_validator_state_updated_to_new_role() {
        let (key_manager, _) = crate::key_manager::test_utils::test_keys();
        let ctx = test_context(key_manager);

        let app = App::new()
            .wrap(Logger::default())
            .app_data(Data::new(ctx.clone()))
            .route("/", web::post().to(set_state::<AppContext<MockHttpClient>>));

        let app = init_service(app).await;

        let req = TestRequest::post()
            .uri("/")
            .insert_header(ContentType::json())
            .set_payload(r#"{"role":"cosigner"}"#)
            .to_request();

        let res = call_service(&app, req).await;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        assert_eq!(ctx.get_validator_state().role(), ValidatorRole::Cosigner)
    }
}
