use crate::model::AppError;
use crate::user::model::{JwtClaims, LoginResponse, NewUser, UserId};
use crate::user::repository::UserRepository;
use anyhow::Context;
use argon2::password_hash::errors::Error::Password as InvalidPassword;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use chrono::prelude::*;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::ops::Add;
use std::time::Duration;

#[async_trait]
pub trait UserService {
    async fn register(&self, user: NewUser) -> Result<UserId, AppError>;
    async fn login(&self, username: String, password: String) -> Result<LoginResponse, AppError>;
    async fn update_password(&self, user_id: UserId, password: String) -> Result<bool, AppError>;
    fn validate_token(&self, token: &str) -> Result<JwtClaims, AppError>;
}

pub struct UserServiceImpl<T: UserRepository> {
    user_repository: T,
    jwt_encoding_key_secret: String,
    jwt_exp_duration: Duration,
}

impl<T: UserRepository> UserServiceImpl<T> {
    pub fn new(user_repository: T, jwt_secret: String, jwt_exp_duration: Duration) -> Self {
        Self { user_repository, jwt_encoding_key_secret: jwt_secret, jwt_exp_duration }
    }
}

#[async_trait]
impl<T: UserRepository + Send + Sync> UserService for UserServiceImpl<T> {
    async fn register(&self, user: NewUser) -> Result<UserId, AppError> {
        // NOTE: Since argon2 hashing is expensive CPU-bound computation, it would be better to
        // spawn it on rayon's thread pool, which is suitable for this type of tasks.
        // But for purposes of this application, it should be OK-ish to use spawn_blocking.
        // Further improvement could be using tokio::sync::Semaphore to limit the number of requests.
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&user.password))
            .await
            .context("Failed to execute password hashing")?;
        Ok(self.user_repository.insert(&user.username, &password_hash?, &user.email).await?)
    }

    async fn login(&self, username: String, password: String) -> Result<LoginResponse, AppError> {
        let (user_id, user_pwd_hash) = self
            .user_repository
            .find_id_and_password_by_username(&username)
            .await?
            .ok_or(AppError::NotFound)?;
        let validation_result =
            tokio::task::spawn_blocking(move || validate_password(&password, &user_pwd_hash))
                .await
                .context("Failed to execute password validation")?;
        validation_result?;
        encode_jwt(user_id, &self.jwt_encoding_key_secret, self.jwt_exp_duration)
    }

    async fn update_password(&self, user_id: UserId, password: String) -> Result<bool, AppError> {
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .context("Failed to execute password hashing")?;
        Ok(self.user_repository.update_password(user_id, &password_hash?).await?)
    }

    fn validate_token(&self, access_token: &str) -> Result<JwtClaims, AppError> {
        let jwt_claims = decode::<JwtClaims>(
            access_token,
            &DecodingKey::from_secret(self.jwt_encoding_key_secret.as_ref()),
            &Validation::default(),
        )?
        .claims;
        Ok(jwt_claims)
    }
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_ref(), &salt)
        .context("Failed to hash password")?
        .to_string();
    Ok(password_hash)
}

fn validate_password(login_password: &str, password_hash: &str) -> Result<(), AppError> {
    let parsed_hash = PasswordHash::new(password_hash).context("Failed to hash password")?;
    let result = Argon2::default().verify_password(login_password.as_ref(), &parsed_hash);
    match result {
        Ok(success) => Ok(success),
        Err(InvalidPassword) => Err(AppError::Unauthorized),
        other => other.context("Failed to verify password").map_err(AppError::from),
    }
}

fn encode_jwt(
    user_id: UserId,
    secret: &str,
    jwt_duration: Duration,
) -> Result<LoginResponse, AppError> {
    let now = Utc::now();
    let iat = now.timestamp() as u64; // safe to cast since current timestamp is always positive
    let exp = now.add(jwt_duration).timestamp() as u64;
    let claims = JwtClaims { sub: user_id, exp, iat };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref()))
        .context("Failed to encode JWT")?;
    Ok(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_duration.as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::repository::MockUserRepository;
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use mockall::predicate::*;
    use uuid::Uuid;

    const JWT_SECRET: &str = "test_secret_12345";
    const JWT_DURATION: Duration = Duration::from_secs(3600);

    #[tokio::test]
    async fn test_register_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let mut mock_repository = MockUserRepository::new();
        let (username, password, email) = ("user1", "test_pass", "test@example.com");
        mock_repository
            .expect_insert()
            .withf(move |insert_name, insert_pass, insert_mail| {
                let matches_hash = validate_password(password, insert_pass).is_ok();
                insert_name == username && insert_mail == email && matches_hash
            })
            .return_once(move |_, _, _| Ok(user_id));
        let service = UserServiceImpl::new(mock_repository, JWT_SECRET.to_string(), JWT_DURATION);

        let user = NewUser {
            username: username.to_string(),
            password: password.to_string(),
            email: email.to_string(),
        };
        let registered_id = service.register(user).await.unwrap();
        assert_eq!(user_id, registered_id);
    }

    #[tokio::test]
    async fn test_login_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let username = "test";
        let password = "test_password";
        let password_hash = hash_password(password).unwrap();
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_find_id_and_password_by_username()
            .with(eq(username))
            .return_once(move |_| Ok(Some((user_id, password_hash))));
        let service = UserServiceImpl::new(mock_repository, JWT_SECRET.to_string(), JWT_DURATION);

        let result = service.login(username.to_string(), password.to_string()).await.unwrap();
        let claims = decode::<JwtClaims>(
            &result.access_token,
            &DecodingKey::from_secret(JWT_SECRET.as_ref()),
            &Validation::default(),
        )
        .unwrap()
        .claims;

        assert_eq!("Bearer", result.token_type);
        assert_eq!(JWT_DURATION.as_secs(), result.expires_in);
        assert_eq!(user_id, claims.sub);
    }

    #[tokio::test]
    async fn test_update_password_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let password = "test_password";
        let mut mock_repository = MockUserRepository::new();
        mock_repository
            .expect_update_password()
            .withf(move |id, pass| {
                let matches_hash = validate_password(password, pass).is_ok();
                id == &user_id && matches_hash
            })
            .return_once(|_, _| Ok(true));

        let service = UserServiceImpl::new(mock_repository, JWT_SECRET.to_string(), JWT_DURATION);
        assert!(service.update_password(user_id, password.to_string()).await.unwrap())
    }

    #[test]
    fn test_validate_valid_token() {
        let user_id = UserId::new(Uuid::new_v4());
        let token = encode_jwt(user_id, JWT_SECRET, JWT_DURATION).unwrap().access_token;
        let service =
            UserServiceImpl::new(MockUserRepository::new(), JWT_SECRET.to_string(), JWT_DURATION);

        let jwt_claims = service.validate_token(&token).unwrap();
        assert_eq!(user_id, jwt_claims.sub);
    }

    #[test]
    fn test_validate_invalid_token() {
        let token = "wrong_token";
        let service =
            UserServiceImpl::new(MockUserRepository::new(), JWT_SECRET.to_string(), JWT_DURATION);

        let result = service.validate_token(&token);
        assert!(matches!(result, Err(AppError::JwtValidation(_))));
    }
}
