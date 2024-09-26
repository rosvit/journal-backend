pub mod common;

use common::{
    channel, clean_up, create_pg_pool, execute_blocking, get_pg_port, start_pg_container, Channel,
    ContainerCommand,
};
use ctor::{ctor, dtor};
use journal_backend::journal::model::{EventType, EventTypeId};
use journal_backend::journal::repository::{EventTypeRepository, PgEventTypeRepository};
use journal_backend::user::model::UserId;
use journal_backend::user::repository::{PgUserRepository, UserRepository};
use lazy_static::lazy_static;
use std::thread;
use uuid::Uuid;

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

    assert_eq!(events, vec![EventType { id, user_id, name: "test_event".to_string(), tags }]);
}

#[tokio::test]
async fn test_insert() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let tags = vec!["tag1".to_string(), "tag2".to_string()];

    let event_id = event_repo.insert(user_id, "test_event", &tags).await.unwrap();

    let event = event_repo.find_by_id(user_id, event_id).await.unwrap().expect("not found");
    assert_eq!(event.user_id, user_id);
    assert_eq!(event.name, "test_event");
    assert_eq!(event.tags, tags);
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
        updated,
        EventType { id, user_id, name: "new_name".to_string(), tags: vec!["new_tag".to_string()] }
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
    assert_eq!(found, None);
}

#[tokio::test]
async fn test_validate() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let event_id = event_repo
        .insert(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();

    let res_false =
        event_repo.validate(user_id, event_id, &vec!["tag_a".to_string()]).await.unwrap();
    let res_true = event_repo.validate(user_id, event_id, &vec!["tag2".to_string()]).await.unwrap();

    assert!(!res_false);
    assert!(res_true);
}

#[tokio::test]
async fn test_validate_empty_tags() {
    let fixture = setup_test().await;
    let event_repo = &fixture.event_repo;
    let user_id = fixture.default_user_id;
    let event_id = event_repo
        .insert(user_id, "test_event", &vec!["tag1".to_string(), "tag2".to_string()])
        .await
        .unwrap();
    let other_event = EventTypeId::new(Uuid::new_v4());

    let res_false = event_repo.validate(user_id, other_event, &vec![]).await.unwrap();
    let res_true = event_repo.validate(user_id, event_id, &vec![]).await.unwrap();

    assert!(!res_false);
    assert!(res_true);
}

struct TestFixture<A: UserRepository, B: EventTypeRepository> {
    user_repo: A,
    event_repo: B,
    default_user_id: UserId,
}

async fn setup_test() -> TestFixture<PgUserRepository, PgEventTypeRepository> {
    let port = get_pg_port(&CMD_IN, &PG_PORT).await;
    let pool = create_pg_pool(port).await;
    let user_repo = PgUserRepository::new(pool.clone());
    let event_repo = PgEventTypeRepository::new(pool.clone());
    let default_user_id = user_repo.insert("default", "default", "default").await.unwrap();

    TestFixture { user_repo, event_repo, default_user_id }
}
