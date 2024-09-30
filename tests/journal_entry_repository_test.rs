pub mod common;

use chrono::Utc;
use common::{
    channel, clean_up, create_pg_pool, execute_blocking, get_pg_port, start_pg_container, Channel,
    ContainerCommand,
};
use ctor::{ctor, dtor};
use journal_backend::journal::model::{EventTypeId, JournalEntry, SearchFilter, SortOrder};
use journal_backend::journal::repository::{
    EventTypeRepository, JournalEntryRepository, PgEventTypeRepository, PgJournalEntryRepository,
};
use journal_backend::user::model::UserId;
use journal_backend::user::repository::{PgUserRepository, UserRepository};
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
async fn test_insert() {
    let fixture = setup_test().await;
    let journal_repo = &fixture.journal_repo;
    let user_id = fixture.default_user_id;
    let event_id = fixture.default_event_type_id;
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let now = Utc::now();

    let id = journal_repo.insert(user_id, event_id, Some("test"), &tags, Some(now)).await.unwrap();

    let entry = journal_repo.find_by_id(user_id, id).await.unwrap().expect("not found");
    let expected = JournalEntry {
        id,
        user_id,
        event_type_id: event_id,
        description: Some("test".to_string()),
        tags,
        created_at: now,
    };
    assert_eq!(expected, entry);
}

#[tokio::test]
async fn test_update() {
    let fixture = setup_test().await;
    let journal_repo = &fixture.journal_repo;
    let user_id = fixture.default_user_id;
    let event_id = fixture.default_event_type_id;
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let now = Utc::now();
    let id =
        journal_repo.insert(user_id, event_id, Some("test"), &Vec::new(), Some(now)).await.unwrap();

    let update_res = journal_repo.update(user_id, id, Some("updated"), &tags).await.unwrap();
    assert!(update_res);

    let entry = journal_repo.find_by_id(user_id, id).await.unwrap().expect("not found");
    let expected = JournalEntry {
        id,
        user_id,
        event_type_id: event_id,
        description: Some("updated".to_string()),
        tags,
        created_at: now,
    };
    assert_eq!(expected, entry);
}

#[tokio::test]
async fn test_delete() {
    let fixture = setup_test().await;
    let journal_repo = &fixture.journal_repo;
    let user_id = fixture.default_user_id;
    let event_id = fixture.default_event_type_id;
    let id = journal_repo.insert(user_id, event_id, Some("test"), &Vec::new(), None).await.unwrap();

    let delete_res = journal_repo.delete(user_id, id).await.unwrap();
    assert!(delete_res);

    let found = journal_repo.find_by_id(user_id, id).await.unwrap();
    assert_eq!(None, found);
}

#[tokio::test]
async fn test_find_empty_filters() {
    let fixture = setup_test().await;
    let journal_repo = &fixture.journal_repo;
    let user_id = fixture.default_user_id;
    let event_type_id = fixture.default_event_type_id;
    let tags = vec!["tag1".to_string()];
    let description = Some("test".to_string());
    let created_at = Utc::now();
    let id = journal_repo
        .insert(user_id, event_type_id, Some("test"), &tags, Some(created_at))
        .await
        .unwrap();

    // entry for other user that shouldn't be found by filter
    let other_user = fixture.user_repo.insert("other", "other", "other").await.unwrap();
    let other_event = fixture.event_repo.insert(other_user, "other", &vec![]).await.unwrap();
    let _ = journal_repo.insert(other_user, other_event, None, &vec![], None).await.unwrap();

    let entries = journal_repo.find(user_id, &SearchFilter::default()).await.unwrap();
    assert_eq!(
        vec![JournalEntry { id, user_id, event_type_id, description, tags, created_at }],
        entries
    );
}

#[tokio::test]
async fn test_find_all_filters() {
    let fixture = setup_test().await;
    let journal_repo = &fixture.journal_repo;
    let user_id = fixture.default_user_id;
    let event_type_id = fixture.default_event_type_id;
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let description = Some("test".to_string());
    let one_minute = Duration::from_secs(60);
    let created_at = Utc::now().sub(one_minute);
    let id = journal_repo
        .insert(user_id, event_type_id, Some("test"), &tags, Some(created_at))
        .await
        .unwrap();

    // entry with other event type that shouldn't be found by filter
    let other_event = fixture.event_repo.insert(user_id, "other", &vec![]).await.unwrap();
    let _ = journal_repo.insert(user_id, other_event, None, &vec![], None).await.unwrap();

    let filter = SearchFilter {
        event_type_id: Some(event_type_id),
        tags: vec!["tag1".to_string()],
        before: Some(Utc::now()),
        after: Some(created_at.sub(one_minute)),
        sort: Some(SortOrder::Desc),
        offset: Some(0),
        limit: Some(10),
    };

    let entries = journal_repo.find(user_id, &filter).await.unwrap();
    assert_eq!(
        vec![JournalEntry { id, user_id, event_type_id, description, tags, created_at }],
        entries
    );
}

#[tokio::test]
async fn test_contains_with_tags() {
    let fixture = setup_test().await;
    let journal_repo = &fixture.journal_repo;
    let user_id = fixture.default_user_id;
    let event_id = fixture.default_event_type_id;
    let tags = vec!["test".to_string(), "tag".to_string(), "super".to_string()];
    let now = Utc::now();

    let _ = journal_repo.insert(user_id, event_id, None, &vec![], None).await.unwrap();
    let _ = journal_repo.insert(user_id, event_id, None, &tags, Some(now)).await.unwrap();

    let should_contain = journal_repo
        .contains_with_tags(event_id, &vec!["tag".to_string(), "super".to_string()])
        .await
        .unwrap();
    let should_not_contain =
        journal_repo.contains_with_tags(event_id, &vec!["other".to_string()]).await.unwrap();
    let empty = journal_repo.contains_with_tags(event_id, &vec![]).await.unwrap();

    assert!(should_contain);
    assert!(!should_not_contain);
    assert!(!empty);
}

struct TestFixture<A: UserRepository, B: EventTypeRepository, C: JournalEntryRepository> {
    user_repo: A,
    event_repo: B,
    journal_repo: C,
    default_user_id: UserId,
    default_event_type_id: EventTypeId,
}

async fn setup_test(
) -> TestFixture<PgUserRepository, PgEventTypeRepository, PgJournalEntryRepository> {
    let port = get_pg_port(&CMD_IN, &PG_PORT).await;
    let pool = create_pg_pool(port).await;
    let user_repo = PgUserRepository::new(pool.clone());
    let event_repo = PgEventTypeRepository::new(pool.clone());
    let journal_repo = PgJournalEntryRepository::new(pool.clone());
    let default_user_id = user_repo.insert("default", "default", "default").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let default_event_type_id =
        event_repo.insert(default_user_id, "default_event", &tags).await.unwrap();

    TestFixture { user_repo, event_repo, journal_repo, default_user_id, default_event_type_id }
}
