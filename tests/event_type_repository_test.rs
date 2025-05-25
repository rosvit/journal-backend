pub mod common;

use common::{
    channel, clean_up, create_pg_pool, execute_blocking, get_pg_port, start_pg_container, Channel,
    ContainerCommand,
};
use ctor::{ctor, dtor};
use journal_backend::journal::model::EventType;
use journal_backend::journal::repository::*;
use journal_backend::model::AppError;
use journal_backend::user::model::UserId;
use journal_backend::user::repository::{PgUserRepository, UserRepository};
use lazy_static::lazy_static;
use std::thread;

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
async fn test_find_by_user_id() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;

    let user_id = fixture.user_repo.insert("user", "password", "email").await.unwrap();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];
    let id = event_repo.insert(user_id, "test_event", &tags).await.unwrap();
    let _ = event_repo.insert(fixture.default_user_id, "other", &vec![]).await.unwrap();

    let events = event_repo.find_by_user_id(user_id).await.unwrap();
    assert_eq!(vec![EventType { id, user_id, name: "test_event".to_string(), tags }], events);
}

#[tokio::test]
async fn test_insert() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let tags = vec!["tag1".to_string(), "tag2".to_string()];

    let event_id = event_repo.insert(user_id, "test_event", &tags).await.unwrap();

    let event = event_repo.find_by_id(user_id, event_id).await.unwrap().expect("not found");
    assert_eq!(user_id, event.user_id);
    assert_eq!("test_event", event.name);
    assert_eq!(tags, event.tags);
}

#[tokio::test]
async fn test_update() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let id = event_repo
        .insert(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();

    event_repo.update(user_id, id, "new_name", &vec!["new_tag".to_string()]).await.unwrap();

    let updated = event_repo.find_by_id(user_id, id).await.unwrap().expect("not found");
    assert_eq!(
        EventType { id, user_id, name: "new_name".to_string(), tags: vec!["new_tag".to_string()] },
        updated
    );
}

#[tokio::test]
async fn test_update_attempt_remove_used_tag() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let id = event_repo
        .insert(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();
    fixture
        .journal_repo
        .insert(user_id, id, Some("test"), &vec!["tag2".to_string()], None)
        .await
        .unwrap();

    let res_err = event_repo.update(user_id, id, "new", &vec!["new".to_string()]).await;
    assert!(matches!(res_err, Err(AppError::TagsStillUsed(_))));
    if let Err(AppError::TagsStillUsed(tags)) = res_err {
        assert_eq!(vec!["tag2".to_string()], tags);
    }
}

#[tokio::test]
async fn test_update_remove_unused_tag() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let id = event_repo
        .insert(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();
    fixture
        .journal_repo
        .insert(user_id, id, Some("test"), &vec!["tag1".to_string()], None)
        .await
        .unwrap();

    event_repo.update(user_id, id, "new", &vec!["tag1".to_string()]).await.unwrap();
    let updated = event_repo.find_by_id(user_id, id).await.unwrap().expect("not found");
    assert_eq!(
        EventType { id, user_id, name: "new".to_string(), tags: vec!["tag1".to_string()] },
        updated
    );
}

#[tokio::test]
async fn test_delete() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let id = event_repo.insert(user_id, "test_event", &vec!["tag1".to_string()]).await.unwrap();

    let delete_res = event_repo.delete(user_id, id).await.unwrap();
    assert!(delete_res);

    let found = event_repo.find_by_id(user_id, id).await.unwrap();
    assert_eq!(None, found);
}

struct TestFixture<U: UserRepository, E: EventTypeRepository, J: JournalEntryRepository> {
    user_repo: U,
    event_repo: E,
    journal_repo: J,
    default_user_id: UserId,
}

async fn setup_test(
) -> TestFixture<PgUserRepository, PgEventTypeRepository, PgJournalEntryRepository> {
    let port = get_pg_port(&CMD_IN, &PG_PORT).await;
    let pool = create_pg_pool(port).await;
    let user_repo = PgUserRepository::new(pool.clone());
    let event_repo = PgEventTypeRepository::new(pool.clone());
    let journal_repo = PgJournalEntryRepository::new(pool.clone());
    let default_user_id = user_repo.insert("default", "default", "default").await.unwrap();

    TestFixture { user_repo, event_repo, journal_repo, default_user_id }
}
