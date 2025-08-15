use crate::model::AppError;
use crate::user::model::UserId;
use crate::user::service::UserService;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{HttpMessage, web};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use log::debug;
use uuid::Uuid;

pub async fn access_token_validator<T: UserService + 'static>(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    let Some(service) = req.app_data::<web::Data<T>>() else {
        return Err((actix_web::error::ErrorInternalServerError("Missing app data"), req));
    };

    match service.validate_token(credentials.token()) {
        Ok(jwt_claims) => {
            req.extensions_mut().insert(jwt_claims.sub);
            Ok(req)
        }
        Err(e) => Err((actix_web::Error::from(e), req)),
    }
}

/// Middleware function to check if the caller can access the requested resource.
/// If both {user_id} path parameter and UserId in request data are present, it checks if they match.
pub async fn validate_caller_id(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    if let (Some(user_id_str), Some(caller_id)) =
        (req.match_info().get("user_id"), req.extensions().get::<UserId>())
    {
        let user_id =
            UserId::new(Uuid::parse_str(user_id_str).map_err(|_| {
                actix_web::error::ErrorBadRequest("Failed to parse_user id from path")
            })?);
        if &user_id != caller_id {
            debug!("User ID does not match with value from access token");
            return Err(actix_web::Error::from(AppError::Unauthorized));
        }
    }

    next.call(req).await
}
