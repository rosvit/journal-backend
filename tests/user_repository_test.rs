pub mod common;

use common::{
    Channel, ContainerCommand, channel, clean_up, create_pg_pool, execute_blocking, get_pg_port,
    start_pg_container,
};
use ctor::{ctor, dtor};
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
async fn test_insert_user() {
    let repo = setup_user_repository().await;

    let (name, pass, email) = ("user", "password", "email");
    let id = repo.insert(name, pass, email).await.unwrap();
    let user_from_db = repo.find_by_id(id).await.unwrap().expect("user not found");

    assert_eq!(name, user_from_db.username);
    assert_eq!(pass, user_from_db.password);
    assert_eq!(email, user_from_db.email);
}

#[tokio::test]
async fn test_find_id_and_password_by_username() {
    let repo = setup_user_repository().await;

    repo.insert("user1", "password1", "email1").await.unwrap();
    repo.insert("user2", "password2", "email2").await.unwrap();

    let found_password =
        repo.find_id_and_password_by_username("user1").await.unwrap().expect("user not found");
    assert_eq!("password1", found_password.1);
}

#[tokio::test]
async fn test_update_password() {
    let repo = setup_user_repository().await;
    let id = repo.insert("user", "old", "email").await.unwrap();
    let user_from_db = repo.find_by_id(id).await.unwrap().expect("user not found");
    assert_eq!("old", user_from_db.password);

    let success = repo.update_password(id, "new").await.unwrap();
    assert_eq!(true, success);
    let user_from_db = repo.find_by_id(id).await.unwrap().expect("user not found");
    assert_eq!("new", user_from_db.password);
}

async fn setup_user_repository() -> impl UserRepository {
    let port = get_pg_port(&CMD_IN, &PG_PORT).await;
    let pool = create_pg_pool(port).await;
    PgUserRepository::new(pool)
}
