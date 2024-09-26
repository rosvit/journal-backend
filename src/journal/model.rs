use crate::model::IdType;
use crate::user::model::UserId;
use chrono::prelude::*;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct EventTypeId(Uuid);

impl EventTypeId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl IdType for EventTypeId {}

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct JournalEntryId(Uuid);

impl JournalEntryId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl IdType for JournalEntryId {}

#[derive(Eq, PartialEq, Serialize, Debug)]
pub struct EventType {
    pub id: EventTypeId,
    pub user_id: UserId,
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Eq, PartialEq, Serialize, Debug, sqlx::FromRow)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub user_id: UserId,
    pub event_type_id: EventTypeId,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Validate)]
pub struct EventTypeData {
    pub name: String,
    #[serde(default)]
    #[validate(custom(function = "validate_tags"))]
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug, Validate)]
pub struct NewJournalEntry {
    pub event_type_id: EventTypeId,
    pub description: Option<String>,
    #[serde(default)]
    #[validate(custom(function = "validate_tags"))]
    pub tags: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Debug, Validate)]
pub struct JournalEntryUpdate {
    pub description: Option<String>,
    #[serde(default)]
    #[validate(custom(function = "validate_tags"))]
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug, Default, Validate)]
#[validate(schema(function = "validate_filters"))]
pub struct SearchFilter {
    pub event_type_id: Option<EventTypeId>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub before: Option<DateTime<Utc>>,
    pub after: Option<DateTime<Utc>>,
    pub sort: Option<SortOrder>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Eq, PartialEq, Deserialize, Debug, derive_more::Display)]
pub enum SortOrder {
    #[display("ASC")]
    #[serde(alias = "asc", alias = "ASC")]
    Asc,
    #[display("DESC")]
    #[serde(alias = "desc", alias = "DESC")]
    Desc,
}

fn validate_tags(tags: &[String]) -> Result<(), ValidationError> {
    if tags.iter().any(|t| t.as_str().trim() == "") {
        Err(ValidationError::new("tags"))
    } else {
        Ok(())
    }
}

fn validate_filters(filter: &SearchFilter) -> Result<(), ValidationError> {
    if let (Some(before), Some(after)) = (filter.before, filter.after) {
        (before <= after).then_some(()).ok_or(ValidationError::new("before, after"))
    } else {
        Ok(())
    }
}
