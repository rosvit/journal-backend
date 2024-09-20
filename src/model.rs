use actix_web::http::{header, StatusCode};
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize, Debug)]
pub struct IdResponse<T> {
    pub id: T,
}

#[derive(derive_more::Debug)]
#[debug("{_0}")]
pub struct InvalidField(pub String);

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("validation error on fields: {0:?}")]
    Validation(Vec<InvalidField>),
    #[error("requested resource not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("could not process request")]
    ProcessingError,
    #[error("some of the removed tags {0:?} are still used in journal entries")]
    TagsStillUsed(Vec<String>),
    #[error(transparent)]
    JwtValidation(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let field_errors =
            errors.field_errors().keys().map(|&k| InvalidField(k.to_string())).collect();
        Self::Validation(field_errors)
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match *self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::JwtValidation(_) => StatusCode::UNAUTHORIZED,
            AppError::TagsStillUsed(_) => StatusCode::CONFLICT,
            AppError::DatabaseError(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            AppError::DatabaseError(sqlx::Error::Database(ref db_err)) => match db_err.kind() {
                sqlx::error::ErrorKind::UniqueViolation => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .insert_header(header::ContentType(mime::TEXT_PLAIN))
            .body(self.to_string())
    }
}

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    pub db_migrate_on_start: bool,
    pub jwt_encoding_key_secret: String,
    pub jwt_exp_duration: Duration,
}
