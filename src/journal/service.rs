use crate::journal::model::*;
use crate::journal::repository::{EventTypeRepository, JournalEntryRepository};
use crate::model::AppError;
use crate::user::model::UserId;
use async_trait::async_trait;

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

pub struct JournalServiceImpl<E: EventTypeRepository, J: JournalEntryRepository> {
    event_repository: E,
    journal_repository: J,
}

impl<E: EventTypeRepository, J: JournalEntryRepository> JournalServiceImpl<E, J> {
    pub fn new(event_repository: E, journal_repository: J) -> Self {
        Self { event_repository, journal_repository }
    }
}

#[async_trait]
impl<E, J> JournalService for JournalServiceImpl<E, J>
where
    E: EventTypeRepository + Send + Sync,
    J: JournalEntryRepository + Send + Sync,
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
    use crate::journal::repository::{MockEventTypeRepository, MockJournalEntryRepository};
    use chrono::Utc;
    use mockall::predicate::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_update_event_type_success() {
        let user_id = UserId::new(Uuid::new_v4());
        let id = EventTypeId::new(Uuid::new_v4());
        let update = EventTypeData { name: "update".to_string(), tags: vec!["tag1".to_string()] };

        let journal_repo = MockJournalEntryRepository::new();
        let mut event_repo = MockEventTypeRepository::new();
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
        let journal_repo = MockJournalEntryRepository::new();
        let mut event_repo = MockEventTypeRepository::new();
        event_repo
            .expect_update()
            .with(eq(user_id), eq(id), eq("update"), eq(vec!["tag1".to_string()]))
            .return_once(|_, _, _, _| Ok(false));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let update = EventTypeData { name: "update".to_string(), tags: vec!["tag1".to_string()] };
        let result = service.update_event_type(user_id, id, update).await;
        assert!(matches!(result, Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn test_update_event_type_removed_tags_used_fails() {
        let user_id = UserId::new(Uuid::new_v4());
        let id = EventTypeId::new(Uuid::new_v4());

        let journal_repo = MockJournalEntryRepository::new();
        let mut event_repo = MockEventTypeRepository::new();
        event_repo
            .expect_update()
            .with(eq(user_id), eq(id), eq("update"), eq(vec!["tag1".to_string()]))
            .return_once(|_, _, _, _| Err(AppError::TagsStillUsed(vec!["tag2".to_string()])));
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
        let event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
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
        let event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        journal_repo
            .expect_insert()
            .return_once(|_, _, _, _, _| Err(AppError::EventTypeValidation));
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
        let event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
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
        let id = JournalEntryId::new(Uuid::new_v4());
        let event_repo = MockEventTypeRepository::new();
        let mut journal_repo = MockJournalEntryRepository::new();
        journal_repo.expect_update().return_once(|_, _, _, _| Err(AppError::EventTypeValidation));
        let service = JournalServiceImpl::new(event_repo, journal_repo);

        let update = JournalEntryUpdate {
            description: Some("test".to_string()),
            tags: vec!["test".to_string()],
        };
        let result = service.update_journal_entry(user_id, id, update).await;
        assert!(matches!(result, Err(AppError::EventTypeValidation)));
    }
}
