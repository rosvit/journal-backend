pub mod common;

use chrono::Utc;
use common::{
    channel, clean_up, create_pg_pool, execute_blocking, get_pg_port, start_pg_container, Channel,
    ContainerCommand,
};
use ctor::{ctor, dtor};
use journal_backend::journal::model::{EventType, JournalEntry, SearchFilter, SortOrder};
use journal_backend::journal::repository::{JournalRepository, PostgresJournalRepository};
use journal_backend::user::repository::{PostgresUserRepository, UserRepository};
use lazy_static::lazy_static;
use std::ops::Sub;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref CMD_IN: Channel<ContainerCommand> = channel();
    static ref PG_PORT: Channel<u16> = channel();
    static ref STOP: Channel<()> = channel();
}

#[ctor]
fn on_startup() {
    thread::spawn(|| execute_blocking(start_pg_container(&CMD_IN, &PG_PORT, &STOP)));
}

#[dtor]
fn on_destroy() {
    clean_up(&CMD_IN, &STOP);
}

#[tokio::test]
async fn test_find_event_types_by_user() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();

    let events = journal_repo.find_event_types_by_user_id(user_id).await.unwrap();

    assert_eq!(events, vec![EventType { id, user_id, name: "test_event".to_string(), tags }]);
}

#[tokio::test]
async fn test_insert_event_type() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let event_id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();

    let event = journal_repo
        .find_event_type_by_id(user_id, event_id)
        .await
        .unwrap()
        .expect("event type not found");

    assert_eq!(event.user_id, user_id);
    assert_eq!(event.name, "test_event");
    assert_eq!(event.tags, tags);
}

#[tokio::test]
async fn test_update_event_type() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let id = journal_repo
        .insert_event_type(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();

    journal_repo
        .update_event_type(user_id, id, "new_name", &vec!["new_tag".to_string()])
        .await
        .unwrap();
    let updated = journal_repo
        .find_event_type_by_id(user_id, id)
        .await
        .unwrap()
        .expect("event type not found");

    assert_eq!(
        updated,
        EventType { id, user_id, name: "new_name".to_string(), tags: vec!["new_tag".to_string()] }
    );
}

#[tokio::test]
async fn test_delete_event_type() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let id = journal_repo
        .insert_event_type(user_id, "test_event", &vec!["tag1".to_string()])
        .await
        .unwrap();

    let delete_res = journal_repo.delete_event_type(user_id, id).await.unwrap();
    assert!(delete_res);
    let found = journal_repo.find_event_type_by_id(user_id, id).await.unwrap();

    assert_eq!(found, None);
}

#[tokio::test]
async fn test_validate_all_tags_present() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let event_id = journal_repo
        .insert_event_type(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();

    let res_false =
        journal_repo.validate_tags(user_id, event_id, &vec!["tag_a".to_string()]).await.unwrap();
    let res_true =
        journal_repo.validate_tags(user_id, event_id, &vec!["tag2".to_string()]).await.unwrap();

    assert!(!res_false);
    assert!(res_true);
}

#[tokio::test]
async fn test_insert_journal_entry() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let event_id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();
    let now = Utc::now();

    let id = journal_repo
        .insert_journal_entry(user_id, event_id, Some("test"), &tags, Some(now))
        .await
        .unwrap();

    let entry =
        journal_repo.find_journal_entry_by_id(user_id, id).await.unwrap().expect("not found");
    let expected = JournalEntry {
        id,
        user_id,
        event_type_id: event_id,
        description: Some("test".to_string()),
        tags,
        created_at: now,
    };
    assert_eq!(entry, expected);
}

#[tokio::test]
async fn test_update_journal_entry() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let event_id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();
    let now = Utc::now();
    let updated_time = now.sub(Duration::from_secs(60));

    let id = journal_repo
        .insert_journal_entry(user_id, event_id, Some("test"), &Vec::new(), Some(now))
        .await
        .unwrap();
    let update_res = journal_repo
        .update_journal_entry(user_id, id, Some("updated"), &tags, Some(updated_time))
        .await
        .unwrap();
    assert!(update_res);

    let entry =
        journal_repo.find_journal_entry_by_id(user_id, id).await.unwrap().expect("not found");
    let expected = JournalEntry {
        id,
        user_id,
        event_type_id: event_id,
        description: Some("updated".to_string()),
        tags,
        created_at: updated_time,
    };
    assert_eq!(entry, expected);
}

#[tokio::test]
async fn test_delete_journal_entry() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let event_id = journal_repo
        .insert_event_type(user_id, "test_event", &vec!["tag1".to_string()])
        .await
        .unwrap();
    let id = journal_repo
        .insert_journal_entry(user_id, event_id, Some("test"), &Vec::new(), None)
        .await
        .unwrap();

    let delete_res = journal_repo.delete_journal_entry(user_id, id).await.unwrap();
    assert!(delete_res);
    let found = journal_repo.find_journal_entry_by_id(user_id, id).await.unwrap();

    assert_eq!(found, None);
}

#[tokio::test]
async fn test_find_journal_entries_empty_filters() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string()];
    let event_type_id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();
    let description = Some("test".to_string());
    let created_at = Utc::now();

    let id = journal_repo
        .insert_journal_entry(user_id, event_type_id, Some("test"), &tags, Some(created_at))
        .await
        .unwrap();

    let entries =
        journal_repo.find_journal_entries(user_id, &SearchFilter::default()).await.unwrap();

    assert_eq!(
        entries,
        vec![JournalEntry { id, user_id, event_type_id, description, tags, created_at }]
    );
}

#[tokio::test]
async fn test_find_journal_entries_all_filters() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let event_type_id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();
    let description = Some("test".to_string());
    let one_minute = Duration::from_secs(60);
    let created_at = Utc::now().sub(one_minute);

    let id = journal_repo
        .insert_journal_entry(user_id, event_type_id, Some("test"), &tags, Some(created_at))
        .await
        .unwrap();

    let filter = SearchFilter {
        event_type_id: Some(event_type_id),
        tags: vec!["tag1".to_string()],
        before: Some(Utc::now()),
        after: Some(created_at.sub(one_minute)),
        sort: Some(SortOrder::Desc),
        offset: Some(0),
        limit: Some(10),
    };
    let entries = journal_repo.find_journal_entries(user_id, &filter).await.unwrap();

    assert_eq!(
        entries,
        vec![JournalEntry { id, user_id, event_type_id, description, tags, created_at }]
    );
}

#[tokio::test]
async fn test_contains_entries_with_tags() {
    let (journal_repo, user_repo) = setup_repositories().await;

    let now = Utc::now();
    let user_id = user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["test".to_string(), "tag".to_string(), "super".to_string()];
    let event_id = journal_repo.insert_event_type(user_id, "test_event", &tags).await.unwrap();

    let _ =
        journal_repo.insert_journal_entry(user_id, event_id, None, &vec![], None).await.unwrap();
    let _ =
        journal_repo.insert_journal_entry(user_id, event_id, None, &tags, Some(now)).await.unwrap();

    let should_contain = journal_repo
        .contains_entries_with_tags(event_id, &vec!["tag".to_string(), "super".to_string()])
        .await
        .unwrap();
    let should_not_contain = journal_repo
        .contains_entries_with_tags(event_id, &vec!["other".to_string()])
        .await
        .unwrap();
    let empty = journal_repo.contains_entries_with_tags(event_id, &vec![]).await.unwrap();

    assert!(should_contain);
    assert!(!should_not_contain);
    assert!(!empty);
}

async fn setup_repositories() -> (impl JournalRepository, impl UserRepository) {
    let port = get_pg_port(&CMD_IN, &PG_PORT).await;
    let pool = create_pg_pool(port).await;
    let user_repo = PostgresUserRepository::new(pool.clone());
    let journal_repo = PostgresJournalRepository::new(pool.clone());
    (journal_repo, user_repo)
}
