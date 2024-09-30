use crate::journal::model::{
    EventType, EventTypeData, EventTypeId, JournalEntry, JournalEntryId, JournalEntryUpdate,
    NewJournalEntry, SearchFilter,
};
use crate::journal::repository::{EventTypeRepository, JournalEntryRepository};
use crate::model::AppError;
use crate::user::model::UserId;
use async_trait::async_trait;
use std::collections::HashSet;

#[async_trait]
pub trait JournalService {
    async fn find_all_event_types(&self, user_id: UserId) -> Result<Vec<EventType>, AppError>;

    async fn find_event_type_by_id(
        &self,
        user_id: UserId,
        id: EventTypeId,
    ) -> Result<EventType, AppError>;

    async fn insert_event_type(
        &self,
        user_id: UserId,
        event_type: EventTypeData,
    ) -> Result<EventTypeId, AppError>;

    async fn update_event_type(
        &self,
        user_id: UserId,
        id: EventTypeId,
        event_type: EventTypeData,
    ) -> Result<(), AppError>;

    async fn delete_event_type(&self, user_id: UserId, id: EventTypeId) -> Result<(), AppError>;

    async fn find_journal_entry_by_id(
        &self,
        user_id: UserId,
        id: JournalEntryId,
    ) -> Result<JournalEntry, AppError>;

    async fn find_journal_entries(
        &self,
        user_id: UserId,
        filter: SearchFilter,
    ) -> Result<Vec<JournalEntry>, AppError>;

    async fn insert_journal_entry(
        &self,
        user_id: UserId,
        entry: NewJournalEntry,
    ) -> Result<JournalEntryId, AppError>;

    async fn update_journal_entry(
        &self,
        user_id: UserId,
        id: JournalEntryId,
        entry: JournalEntryUpdate,
    ) -> Result<(), AppError>;

    async fn delete_journal_entry(
        &self,
        user_id: UserId,
        id: JournalEntryId,
    ) -> Result<(), AppError>;
}

pub struct JournalServiceImpl<A: EventTypeRepository, B: JournalEntryRepository> {
    event_repository: A,
    journal_repository: B,
}

impl<A: EventTypeRepository, B: JournalEntryRepository> JournalServiceImpl<A, B> {
    pub fn new(event_repository: A, journal_repository: B) -> Self {
        Self { event_repository, journal_repository }
    }

    async fn validate_event_type(
        &self,
        user_id: UserId,
        id: EventTypeId,
        tags: &[String],
    ) -> Result<(), AppError> {
        let valid = self.event_repository.validate(user_id, id, tags).await?;
        if valid {
            Ok(())
        } else {
            Err(AppError::EventTypeValidation)
        }
    }
}

#[async_trait]
impl<A, B> JournalService for JournalServiceImpl<A, B>
where
    A: EventTypeRepository + Send + Sync,
    B: JournalEntryRepository + Send + Sync,
{
    async fn find_all_event_types(&self, user_id: UserId) -> Result<Vec<EventType>, AppError> {
        Ok(self.event_repository.find_by_user_id(user_id).await?)
    }

    async fn find_event_type_by_id(
        &self,
        user_id: UserId,
        id: EventTypeId,
    ) -> Result<EventType, AppError> {
        self.event_repository.find_by_id(user_id, id).await?.ok_or(AppError::NotFound)
    }

    async fn insert_event_type(
        &self,
        user_id: UserId,
        event_type: EventTypeData,
    ) -> Result<EventTypeId, AppError> {
        let inserted_id =
            self.event_repository.insert(user_id, &event_type.name, &event_type.tags).await?;
        Ok(inserted_id)
    }

    async fn update_event_type(
        &self,
        user_id: UserId,
        id: EventTypeId,
        event_type: EventTypeData,
    ) -> Result<(), AppError> {
        let current =
            self.event_repository.find_by_id(user_id, id).await?.ok_or(AppError::NotFound)?;

        // check if any of the removed tags is still referenced. If yes, return an error.
        let old_tags: HashSet<_> = HashSet::from_iter(&current.tags);
        let new_tags: HashSet<_> = HashSet::from_iter(&event_type.tags);
        let removed: Vec<_> = old_tags.difference(&new_tags).map(|&s| s.to_string()).collect();
        if !removed.is_empty() {
            let in_use = self.journal_repository.contains_with_tags(id, &removed).await?;
            if in_use {
                return Err(AppError::TagsStillUsed(removed));
            }
        }

        self.event_repository
            .update(user_id, id, &event_type.name, &event_type.tags)
            .await?
            .then_some(())
            .ok_or(AppError::NotFound)
    }

    async fn delete_event_type(&self, user_id: UserId, id: EventTypeId) -> Result<(), AppError> {
        self.event_repository.delete(user_id, id).await?.then_some(()).ok_or(AppError::NotFound)
    }

    async fn find_journal_entry_by_id(
        &self,
        user_id: UserId,
        id: JournalEntryId,
    ) -> Result<JournalEntry, AppError> {
        self.journal_repository.find_by_id(user_id, id).await?.ok_or(AppError::NotFound)
    }

    async fn find_journal_entries(
        &self,
        user_id: UserId,
        filter: SearchFilter,
    ) -> Result<Vec<JournalEntry>, AppError> {
        Ok(self.journal_repository.find(user_id, &filter).await?)
    }

    async fn insert_journal_entry(
        &self,
        user_id: UserId,
        entry: NewJournalEntry,
    ) -> Result<JournalEntryId, AppError> {
        self.validate_event_type(user_id, entry.event_type_id, &entry.tags).await?;

        let entry_id = self
            .journal_repository
            .insert(
                user_id,
                entry.event_type_id,
                entry.description.as_deref(),
                &entry.tags,
                entry.created_at,
            )
            .await?;
        Ok(entry_id)
    }

    async fn update_journal_entry(
        &self,
        user_id: UserId,
        id: JournalEntryId,
        update: JournalEntryUpdate,
    ) -> Result<(), AppError> {
        let current =
            self.journal_repository.find_by_id(user_id, id).await?.ok_or(AppError::NotFound)?;
        self.validate_event_type(user_id, current.event_type_id, &update.tags).await?;

        self.journal_repository
            .update(user_id, id, update.description.as_deref(), &update.tags)
            .await?
            .then_some(())
            .ok_or(AppError::NotFound)
    }

    async fn delete_journal_entry(
        &self,
        user_id: UserId,
        id: JournalEntryId,
    ) -> Result<(), AppError> {
        self.journal_repository.delete(user_id, id).await?.then_some(()).ok_or(AppError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::model::EventType;
    use crate::journal::repository::{MockEventTypeRepository, MockJournalEntryRepository};
    use chrono::Utc;
    use mockall::predicate::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_update_event_type_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let id = EventTypeId::new(Uuid::new_v4());
        let et = EventType {
            id,
            user_id,
            name: "test".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        let update = EventTypeData { name: "update".to_string(), tags: vec!["tag1".to_string()] };

        let mut event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        event_repo.expect_find_by_id().with(eq(user_id), eq(id)).return_once(|_, _| Ok(Some(et)));
        journal_repo
            .expect_contains_with_tags()
            .with(eq(id), eq(vec!["tag2".to_string()]))
            .return_once(|_, _| Ok(false));
        event_repo
            .expect_update()
            .with(eq(user_id), eq(id), eq(update.name.clone()), eq(update.tags.clone()))
            .return_once(|_, _, _, _| Ok(true));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let result = service.update_event_type(user_id, id, update).await;
        assert!(matches!(result, Ok(_)));
    }

    #[tokio::test]
    async fn test_update_event_type_not_found_fails() {
        let user_id = UserId::new(Uuid::new_v4());
        let id = EventTypeId::new(Uuid::new_v4());
        let mut event_repo = MockEventTypeRepository::new();
        let journal_repo = MockJournalEntryRepository::new();
        event_repo.expect_find_by_id().with(eq(user_id), eq(id)).return_once(|_, _| Ok(None));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let update = EventTypeData { name: "update".to_string(), tags: vec!["tag1".to_string()] };
        let result = service.update_event_type(user_id, id, update).await;
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn test_update_event_type_removed_tags_used_fails() {
        let user_id = UserId::new(Uuid::new_v4());
        let id = EventTypeId::new(Uuid::new_v4());
        let et = EventType {
            id,
            user_id,
            name: "test".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };

        let mut event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        event_repo.expect_find_by_id().with(eq(user_id), eq(id)).return_once(|_, _| Ok(Some(et)));
        journal_repo
            .expect_contains_with_tags()
            .with(eq(id), eq(vec!["tag2".to_string()]))
            .return_once(|_, _| Ok(true));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let update = EventTypeData { name: "update".to_string(), tags: vec!["tag1".to_string()] };
        let result = service.update_event_type(user_id, id, update).await;
        assert!(matches!(result, Err(AppError::TagsStillUsed(_))));
    }

    #[tokio::test]
    async fn test_insert_journal_entry_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let event_type_id = EventTypeId::new(Uuid::new_v4());
        let id = JournalEntryId::new(Uuid::new_v4());
        let mut event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        event_repo
            .expect_validate()
            .with(eq(user_id), eq(event_type_id), eq(vec!["test".to_string()]))
            .return_once(|_, _, _| Ok(true));
        journal_repo
            .expect_insert()
            .withf(move |uid, eid, desc, tags, _| {
                uid == &user_id
                    && eid == &event_type_id
                    && desc == &Some("test")
                    && tags == &vec!["test".to_string()]
            })
            .return_once(move |_, _, _, _, _| Ok(id));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let entry = NewJournalEntry {
            event_type_id,
            description: Some("test".to_string()),
            tags: vec!["test".to_string()],
            created_at: None,
        };
        let result = service.insert_journal_entry(user_id, entry).await.unwrap();
        assert_eq!(id, result);
    }

    #[tokio::test]
    async fn test_insert_journal_entry_wrong_event_type_fails() {
        let user_id = UserId::new(Uuid::new_v4());
        let event_type_id = EventTypeId::new(Uuid::new_v4());
        let mut event_repo = MockEventTypeRepository::new();
        let journal_repo = MockJournalEntryRepository::new();
        event_repo
            .expect_validate()
            .with(eq(user_id), eq(event_type_id), eq(vec!["test".to_string()]))
            .return_once(|_, _, _| Ok(false));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let entry = NewJournalEntry {
            event_type_id,
            description: Some("test".to_string()),
            tags: vec!["test".to_string()],
            created_at: None,
        };
        let result = service.insert_journal_entry(user_id, entry).await;
        assert!(matches!(result, Err(AppError::EventTypeValidation)));
    }

    #[tokio::test]
    async fn test_update_journal_entry_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let event_type_id = EventTypeId::new(Uuid::new_v4());
        let id = JournalEntryId::new(Uuid::new_v4());
        let current_entry = JournalEntry {
            id,
            user_id,
            event_type_id,
            description: None,
            tags: vec![],
            created_at: Utc::now(),
        };
        let mut event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        event_repo
            .expect_validate()
            .with(eq(user_id), eq(event_type_id), eq(vec!["test".to_string()]))
            .return_once(|_, _, _| Ok(true));
        journal_repo
            .expect_find_by_id()
            .with(eq(user_id), eq(id))
            .return_once(|_, _| Ok(Some(current_entry)));
        journal_repo
            .expect_update()
            .withf(move |uid, eid, desc, tags| {
                uid == &user_id
                    && eid == &id
                    && desc == &Some("test")
                    && tags == vec!["test".to_string()]
            })
            .return_once(|_, _, _, _| Ok(true));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let update = JournalEntryUpdate {
            description: Some("test".to_string()),
            tags: vec!["test".to_string()],
        };
        let result = service.update_journal_entry(user_id, id, update).await;
        assert!(matches!(result, Ok(_)));
    }

    #[tokio::test]
    async fn test_update_journal_entry_wrong_event_type_fails() {
        let user_id = UserId::new(Uuid::new_v4());
        let event_type_id = EventTypeId::new(Uuid::new_v4());
        let id = JournalEntryId::new(Uuid::new_v4());
        let current_entry = JournalEntry {
            id,
            user_id,
            event_type_id,
            description: None,
            tags: vec![],
            created_at: Utc::now(),
        };
        let mut event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        event_repo
            .expect_validate()
            .with(eq(user_id), eq(event_type_id), eq(vec!["test".to_string()]))
            .return_once(|_, _, _| Ok(false));
        journal_repo
            .expect_find_by_id()
            .with(eq(user_id), eq(id))
            .return_once(|_, _| Ok(Some(current_entry)));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let update = JournalEntryUpdate {
            description: Some("test".to_string()),
            tags: vec!["test".to_string()],
        };
        let result = service.update_journal_entry(user_id, id, update).await;
        assert!(matches!(result, Err(AppError::EventTypeValidation)));
    }
}
