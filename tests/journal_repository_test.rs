pub mod common;

use common::{
    channel, clean_up, create_pg_pool, execute_blocking, get_pg_port, start_pg_container, Channel,
    ContainerCommand,
};
use ctor::{ctor, dtor};
use journal_backend::journal::model::EventType;
use journal_backend::journal::repository::{JournalRepository, PostgresJournalRepository};
use journal_backend::user::repository::{PostgresUserRepository, UserRepository};
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

    let event =
        journal_repo.find_event_type_by_id(event_id).await.unwrap().expect("event type not found");

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

    journal_repo.update_event_type(id, "new_name", &vec!["new_tag".to_string()]).await.unwrap();
    let updated =
        journal_repo.find_event_type_by_id(id).await.unwrap().expect("event type not found");

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

    journal_repo.delete_event_type(id).await.unwrap();
    let found = journal_repo.find_event_type_by_id(id).await.unwrap();

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

    let res_false = journal_repo.validate_tags(event_id, &vec!["tag_a".to_string()]).await.unwrap();
    let res_true = journal_repo.validate_tags(event_id, &vec!["tag2".to_string()]).await.unwrap();

    assert!(!res_false);
    assert!(res_true);
}

async fn setup_repositories() -> (impl JournalRepository, impl UserRepository) {
    let port = get_pg_port(&CMD_IN, &PG_PORT).await;
    let pool = create_pg_pool(port).await;
    let user_repo = PostgresUserRepository::new(pool.clone());
    let journal_repo = PostgresJournalRepository::new(pool.clone());
    (journal_repo, user_repo)
}
