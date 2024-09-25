use crate::model::IdType;
use crate::user::model::UserId;
use chrono::prelude::*;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Deserialize, Debug)]
pub struct EventTypeData {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug, Default)]
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
