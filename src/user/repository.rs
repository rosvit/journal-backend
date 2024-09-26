use crate::user::model::{User, UserId};
use async_trait::async_trait;
use sqlx::PgPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, sqlx::Error>;

    async fn find_id_and_password_by_username(
        &self,
        username: &str,
    ) -> Result<Option<(UserId, String)>, sqlx::Error>;

    async fn insert(
        &self,
        username: &str,
        password: &str,
        email: &str,
    ) -> Result<UserId, sqlx::Error>;

    async fn update_password(&self, id: UserId, new_password: &str) -> Result<bool, sqlx::Error>;
}

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT id as "id: _", username, password, email FROM users WHERE id = $1"#,
            id as UserId
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_id_and_password_by_username(
        &self,
        username: &str,
    ) -> Result<Option<(UserId, String)>, sqlx::Error> {
        sqlx::query!(
            r#"SELECT id as "id: UserId", password FROM users WHERE username = $1"#,
            username
        )
        .fetch_optional(&self.pool)
        .await
        .map(|maybe_record| maybe_record.map(|record| (record.id, record.password)))
    }

    async fn insert(
        &self,
        username: &str,
        password: &str,
        email: &str,
    ) -> Result<UserId, sqlx::Error> {
        sqlx::query!(r#"INSERT INTO users (username, password, email) VALUES ($1, $2, $3) RETURNING id as "id: UserId""#,
            username, password, email)
            .fetch_one(&self.pool)
            .await
            .map(|record| record.id)
    }

    async fn update_password(&self, id: UserId, password: &str) -> Result<bool, sqlx::Error> {
        sqlx::query!(r#"UPDATE users SET password = $1 WHERE id = $2"#, password, id as UserId)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
    }
}
