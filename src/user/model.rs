use crate::model::IdType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct UserId(Uuid);

impl UserId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl IdType for UserId {}

#[derive(Debug)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Deserialize, Validate, Debug)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Deserialize, Debug)]
pub struct UpdatePasswordRequest {
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JwtClaims {
    pub sub: UserId,
    pub exp: u64,
    pub iat: u64,
}
