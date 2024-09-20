use crate::journal::model::{EventType, EventTypeData, EventTypeId};
use crate::journal::repository::JournalRepository;
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
}

pub struct JournalServiceImpl<T: JournalRepository> {
    journal_repository: T,
}

impl<T: JournalRepository> JournalServiceImpl<T> {
    pub fn new(journal_repository: T) -> Self {
        Self { journal_repository }
    }
}

#[async_trait]
impl<T: JournalRepository + Send + Sync> JournalService for JournalServiceImpl<T> {
    async fn find_all_event_types(&self, user_id: UserId) -> Result<Vec<EventType>, AppError> {
        Ok(self.journal_repository.find_event_types_by_user_id(user_id).await?)
    }

    async fn find_event_type_by_id(
        &self,
        user_id: UserId,
        id: EventTypeId,
    ) -> Result<EventType, AppError> {
        self.journal_repository.find_event_type_by_id(user_id, id).await?.ok_or(AppError::NotFound)
    }

    async fn insert_event_type(
        &self,
        user_id: UserId,
        event_type: EventTypeData,
    ) -> Result<EventTypeId, AppError> {
        let inserted_id = self
            .journal_repository
            .insert_event_type(user_id, &event_type.name, &event_type.tags)
            .await?;
        Ok(inserted_id)
    }

    async fn update_event_type(
        &self,
        user_id: UserId,
        id: EventTypeId,
        event_type: EventTypeData,
    ) -> Result<(), AppError> {
        let current = self
            .journal_repository
            .find_event_type_by_id(user_id, id)
            .await?
            .ok_or(AppError::NotFound)?;

        // check if any of the removed tags is still referenced. If yes, return an error.
        let old_tags: HashSet<_> = HashSet::from_iter(&current.tags);
        let new_tags: HashSet<_> = HashSet::from_iter(&event_type.tags);
        let removed: Vec<_> = old_tags.difference(&new_tags).map(|&s| s.to_string()).collect();
        if !removed.is_empty() {
            let in_use = self.journal_repository.contains_entries_with_tags(id, &removed).await?;
            if in_use {
                return Err(AppError::TagsStillUsed(removed));
            }
        }

        self.journal_repository
            .update_event_type(user_id, id, &event_type.name, &event_type.tags)
            .await?
            .then_some(())
            .ok_or(AppError::NotFound)
    }

    async fn delete_event_type(&self, user_id: UserId, id: EventTypeId) -> Result<(), AppError> {
        self.journal_repository
            .delete_event_type(user_id, id)
            .await?
            .then_some(())
            .ok_or(AppError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::model::EventType;
    use crate::journal::repository::MockJournalRepository;
    use mockall::predicate::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_update_event_type_not_found_fails() {
        let user_id = UserId::new(Uuid::new_v4());
        let id = EventTypeId::new(Uuid::new_v4());
        let mut repo = MockJournalRepository::new();
        repo.expect_find_event_type_by_id().with(eq(user_id), eq(id)).return_once(|_, _| Ok(None));
        let service = JournalServiceImpl::new(repo);

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

        let mut repo = MockJournalRepository::new();
        repo.expect_find_event_type_by_id()
            .with(eq(user_id), eq(id))
            .return_once(|_, _| Ok(Some(et)));
        repo.expect_contains_entries_with_tags()
            .with(eq(id), eq(vec!["tag2".to_string()]))
            .return_once(|_, _| Ok(true));
        let service = JournalServiceImpl::new(repo);

        let update = EventTypeData { name: "update".to_string(), tags: vec!["tag1".to_string()] };
        let result = service.update_event_type(user_id, id, update).await;
        assert!(matches!(result, Err(AppError::TagsStillUsed(_))));
    }

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

        let mut repo = MockJournalRepository::new();
        repo.expect_find_event_type_by_id()
            .with(eq(user_id), eq(id))
            .return_once(|_, _| Ok(Some(et)));
        repo.expect_contains_entries_with_tags()
            .with(eq(id), eq(vec!["tag2".to_string()]))
            .return_once(|_, _| Ok(false));
        repo.expect_update_event_type()
            .with(eq(user_id), eq(id), eq(update.name.clone()), eq(update.tags.clone()))
            .return_once(|_, _, _, _| Ok(true));
        let service = JournalServiceImpl::new(repo);

        let result = service.update_event_type(user_id, id, update).await;
        assert!(matches!(result, Ok(_)));
    }
}
