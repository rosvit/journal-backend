mod common;

use common::{create_pg_pool, start_pg_container, DEFAULT_PG_PORT};
use journal_backend::user::repository::{PostgresUserRepository, UserRepository};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::ContainerAsync;

#[tokio::test]
async fn test_insert_user() {
    let (_container, repo) = setup_user_repository().await;

    let (name, pass, email) = ("user", "password", "email");
    let id = repo.insert(name, pass, email).await.unwrap();
    let user_from_db = repo.find_by_id(id).await.unwrap().expect("user not found");

    assert_eq!(user_from_db.username, name);
    assert_eq!(user_from_db.password, pass);
    assert_eq!(user_from_db.email, email);
}

#[tokio::test]
async fn test_find_id_and_password_by_username() {
    let (_container, repo) = setup_user_repository().await;

    repo.insert("user1", "password1", "email1").await.unwrap();
    repo.insert("user2", "password2", "email2").await.unwrap();
    let found_password =
        repo.find_id_and_password_by_username("user1").await.unwrap().expect("user not found");

    assert_eq!(found_password.1, "password1");
}

#[tokio::test]
async fn test_update_password() {
    let (_container, repo) = setup_user_repository().await;
    let id = repo.insert("user", "old", "email").await.unwrap();
    let user_from_db = repo.find_by_id(id).await.unwrap().expect("user not found");
    assert_eq!(user_from_db.password, "old");

    let success = repo.update_password(id, "new").await.unwrap();
    assert_eq!(success, true);
    let user_from_db = repo.find_by_id(id).await.unwrap().expect("user not found");
    assert_eq!(user_from_db.password, "new");
}

async fn setup_user_repository() -> (ContainerAsync<Postgres>, impl UserRepository) {
    let container = start_pg_container().await;
    let host_port = container.get_host_port_ipv4(DEFAULT_PG_PORT).await.unwrap();
    let pool = create_pg_pool(host_port).await;
    let repo = PostgresUserRepository::new(pool);
    (container, repo)
}
