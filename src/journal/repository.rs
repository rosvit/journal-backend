use crate::journal::model::{EventType, EventTypeId, JournalEntry, JournalEntryId, SearchFilter};
use crate::user::model::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EventTypeRepository {
    async fn find_by_id(
        &self,
        user_id: UserId,
        id: EventTypeId,
    ) -> Result<Option<EventType>, sqlx::Error>;

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<EventType>, sqlx::Error>;

    async fn insert(
        &self,
        user_id: UserId,
        name: &str,
        tags: &[String],
    ) -> Result<EventTypeId, sqlx::Error>;

    async fn update(
        &self,
        user_id: UserId,
        id: EventTypeId,
        name: &str,
        tags: &[String],
    ) -> Result<bool, sqlx::Error>;

    async fn delete(&self, user_id: UserId, id: EventTypeId) -> Result<bool, sqlx::Error>;

    async fn validate(
        &self,
        user_id: UserId,
        id: EventTypeId,
        tags: &[String],
    ) -> Result<bool, sqlx::Error>;
}

pub struct PgEventTypeRepository {
    pool: PgPool,
}

impl PgEventTypeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventTypeRepository for PgEventTypeRepository {
    async fn find_by_id(
        &self,
        user_id: UserId,
        id: EventTypeId,
    ) -> Result<Option<EventType>, sqlx::Error> {
        sqlx::query_as!(
            EventType,
            r#"SELECT id as "id: _", user_id as "user_id: _", name, tags FROM event_type
                WHERE id = $1 AND user_id = $2"#,
            id as EventTypeId,
            user_id as UserId
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<EventType>, sqlx::Error> {
        sqlx::query_as!(
            EventType,
            r#"SELECT id as "id: _", user_id as "user_id: _", name, tags FROM event_type WHERE user_id = $1"#,
            user_id as UserId,
        )
            .fetch_all(&self.pool)
            .await
    }

    async fn insert(
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

    async fn update(
        &self,
        user_id: UserId,
        id: EventTypeId,
        name: &str,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            r#"UPDATE event_type SET name = $1, tags = $2 WHERE id = $3 AND user_id = $4"#,
            name,
            tags,
            id as EventTypeId,
            user_id as UserId
        )
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() > 0)
    }

    async fn delete(&self, user_id: UserId, id: EventTypeId) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            r#"DELETE FROM event_type WHERE id = $1 and user_id = $2"#,
            id as EventTypeId,
            user_id as UserId
        )
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() > 0)
    }

    async fn validate(
        &self,
        user_id: UserId,
        id: EventTypeId,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        let mut query: QueryBuilder<Postgres> =
            QueryBuilder::new(r#"SELECT count(id) FROM event_type WHERE id = "#);
        query.push_bind(id);
        query.push(" AND user_id = ").push_bind(user_id);

        if !tags.is_empty() {
            query.push(" AND ").push_bind(tags).push(" <@ tags");
        }

        query.build().fetch_one(&self.pool).await.map(|row| {
            let count: i64 = row.try_get("count").unwrap_or(0);
            count.is_positive()
        })
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait JournalEntryRepository {
    async fn find_by_id(
        &self,
        user_id: UserId,
        id: JournalEntryId,
    ) -> Result<Option<JournalEntry>, sqlx::Error>;

    async fn find(
        &self,
        user_id: UserId,
        filter: &SearchFilter,
    ) -> Result<Vec<JournalEntry>, sqlx::Error>;

    async fn insert<'a>(
        &self,
        user_id: UserId,
        event_type_id: EventTypeId,
        description: Option<&'a str>,
        tags: &[String],
        created_at: Option<DateTime<Utc>>,
    ) -> Result<JournalEntryId, sqlx::Error>;

    async fn update<'a>(
        &self,
        user_id: UserId,
        id: JournalEntryId,
        description: Option<&'a str>,
        tags: &[String],
    ) -> Result<bool, sqlx::Error>;

    async fn delete(&self, user_id: UserId, id: JournalEntryId) -> Result<bool, sqlx::Error>;

    async fn contains_with_tags(
        &self,
        event_type_id: EventTypeId,
        tags: &[String],
    ) -> Result<bool, sqlx::Error>;
}

pub struct PgJournalEntryRepository {
    pool: PgPool,
}

impl PgJournalEntryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JournalEntryRepository for PgJournalEntryRepository {
    async fn find_by_id(
        &self,
        user_id: UserId,
        id: JournalEntryId,
    ) -> Result<Option<JournalEntry>, sqlx::Error> {
        sqlx::query_as!(
            JournalEntry,
            r#"SELECT id as "id: _", user_id as "user_id: _", event_type_id as "event_type_id: _",
                description, tags, created_at
                FROM journal_entry WHERE id = $1 AND user_id = $2"#,
            id as JournalEntryId,
            user_id as UserId
        )
        .fetch_optional(&self.pool)
        .await
    }

    async fn find(
        &self,
        user_id: UserId,
        filter: &SearchFilter,
    ) -> Result<Vec<JournalEntry>, sqlx::Error> {
        let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"SELECT id, user_id, event_type_id, description, tags, created_at
                FROM journal_entry WHERE user_id = "#,
        );
        query.push_bind(user_id);

        if let Some(id) = &filter.event_type_id {
            query.push(" AND event_type_id = ").push_bind(id);
        };
        if !filter.tags.is_empty() {
            query.push(" AND tags @> ").push_bind(&filter.tags);
        };
        if let Some(before) = &filter.before {
            query.push(" AND created_at <= ").push_bind(before);
        };
        if let Some(after) = &filter.after {
            query.push(" AND created_at >= ").push_bind(after);
        };
        if let Some(sort) = &filter.sort {
            query.push(" ORDER BY created_at ").push(sort);
        };
        if let Some(offset) = filter.offset {
            query.push(" OFFSET ").push(offset);
        };
        if let Some(limit) = filter.limit {
            query.push(" LIMIT ").push(limit);
        };

        query.build_query_as::<JournalEntry>().fetch_all(&self.pool).await
    }

    async fn insert<'a>(
        &self,
        user_id: UserId,
        event_type_id: EventTypeId,
        description: Option<&'a str>,
        tags: &[String],
        created_at: Option<DateTime<Utc>>,
    ) -> Result<JournalEntryId, sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO journal_entry (user_id, event_type_id, description, tags, created_at)
                VALUES ($1, $2, $3, $4, $5) RETURNING id as "id: JournalEntryId""#,
            user_id as UserId,
            event_type_id as EventTypeId,
            description,
            tags,
            created_at.unwrap_or(Utc::now())
        )
        .fetch_one(&self.pool)
        .await
        .map(|record| record.id)
    }

    async fn update<'a>(
        &self,
        user_id: UserId,
        id: JournalEntryId,
        description: Option<&'a str>,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            r#"UPDATE journal_entry SET description = $1, tags = $2 WHERE id = $3 AND user_id = $4"#,
            description,
            tags,
            id as JournalEntryId,
            user_id as UserId
        )
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() > 0)
    }

    async fn delete(&self, user_id: UserId, id: JournalEntryId) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            r#"DELETE FROM journal_entry WHERE id = $1 and user_id = $2"#,
            id as JournalEntryId,
            user_id as UserId
        )
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected() > 0)
    }

    async fn contains_with_tags(
        &self,
        event_type_id: EventTypeId,
        tags: &[String],
    ) -> Result<bool, sqlx::Error> {
        if tags.is_empty() {
            return Ok(false);
        }

        sqlx::query!(
            r#"SELECT count(id) FROM journal_entry WHERE event_type_id = $1 AND tags && $2"#,
            event_type_id as EventTypeId,
            tags
        )
        .fetch_one(&self.pool)
        .await
        .map(|record| record.count.unwrap_or(0).is_positive())
    }
}
