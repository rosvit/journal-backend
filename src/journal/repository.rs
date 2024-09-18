use crate::journal::model::{EventType, EventTypeId};
use crate::user::model::UserId;
use async_trait::async_trait;
use sqlx::PgPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait JournalRepository {
    async fn find_event_type_by_id(
        &self,
        event_type_id: EventTypeId,
    ) -> Result<Option<EventType>, sqlx::Error>;

    async fn find_event_types_by_user_id(
        &self,
        user_id: UserId,
    ) -> Result<Vec<EventType>, sqlx::Error>;

    async fn insert_event_type(
        &self,
        user_id: UserId,
        name: &str,
        tags: &[String],
    ) -> Result<EventTypeId, sqlx::Error>;

    async fn update_event_type(
        &self,
        event_type_id: EventTypeId,
        name: &str,
        tags: &[String],
    ) -> Result<(), sqlx::Error>;

    async fn delete_event_type(&self, event_type_id: EventTypeId) -> Result<(), sqlx::Error>;

    async fn validate_tags(
        &self,
        event_type_id: EventTypeId,
        tags: &[String],
    ) -> Result<bool, sqlx::Error>;
}

pub struct PostgresJournalRepository {
    pool: PgPool,
}

impl PostgresJournalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JournalRepository for PostgresJournalRepository {
    async fn find_event_type_by_id(
        &self,
        event_type_id: EventTypeId,
    ) -> Result<Option<EventType>, sqlx::Error> {
        sqlx::query_as!(
            EventType,
            r#"SELECT id as "id: _", user_id as "user_id: _", name, tags FROM event_type WHERE id = $1"#,
            event_type_id as EventTypeId,
        )
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_event_types_by_user_id(
        &self,
        user_id: UserId,
    ) -> Result<Vec<EventType>, sqlx::Error> {
        sqlx::query_as!(
            EventType,
            r#"SELECT id as "id: _", user_id as "user_id: _", name, tags FROM event_type WHERE user_id = $1"#,
            user_id as UserId,
        )
            .fetch_all(&self.pool)
            .await
    }

    async fn insert_event_type(
        &self,
        user_id: UserId,
        name: &str,
        tags: &[String],
    ) -> Result<EventTypeId, sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO event_type (user_id, name, tags) VALUES ($1, $2, $3) RETURNING id as "id: EventTypeId""#,
            user_id as UserId, name, tags)
            .fetch_one(&self.pool)
            .await
            .map(|record| record.id)
    }

    async fn update_event_type(
        &self,
        event_type_id: EventTypeId,
        name: &str,
        tags: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE event_type SET name = $1, tags = $2 WHERE id = $3"#,
            name,
            tags,
            event_type_id as EventTypeId
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn delete_event_type(&self, event_type_id: EventTypeId) -> Result<(), sqlx::Error> {
        sqlx::query!(r#"DELETE FROM event_type WHERE id = $1"#, event_type_id as EventTypeId)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }

    async fn validate_tags(
        &self,
        event_type_id: EventTypeId,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            r#"SELECT count(id) FROM event_type WHERE id = $1 AND $2 <@ tags"#,
            event_type_id as EventTypeId,
            tags
        )
        .fetch_one(&self.pool)
        .await
        .map(|record| record.count.unwrap_or(0).is_positive())
    }
}
