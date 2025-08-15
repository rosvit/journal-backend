use crate::model::{AppError, IdResponse};
use crate::user::model::{LoginRequest, NewUser, UpdatePasswordRequest, UserId};
use crate::user::service::UserService;
use actix_web::http::header;
use actix_web::http::header::CacheDirective;
use actix_web::{HttpResponse, web};
use validator::Validate;

pub async fn register<T: UserService>(
    user: web::Json<NewUser>,
    user_service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    user.validate().map_err(AppError::from)?;
    let user_id = user_service.register(user.into_inner()).await?;
    Ok(HttpResponse::Ok().json(IdResponse { id: user_id }))
}

pub async fn login<T: UserService>(
    login_data: web::Json<LoginRequest>,
    user_service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let login = login_data.into_inner();
    let login_response = user_service.login(login.username, login.password).await?;
    Ok(HttpResponse::Ok()
        .insert_header(header::CacheControl(vec![CacheDirective::NoStore]))
        .json(login_response))
}

pub async fn update_password<T: UserService>(
    user_id: web::Path<UserId>,
    update: web::Json<UpdatePasswordRequest>,
    user_service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let success =
        user_service.update_password(user_id.into_inner(), update.into_inner().password).await?;
    if success { Ok(HttpResponse::Ok().finish()) } else { Err(AppError::ProcessingError) }
}
