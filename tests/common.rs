use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};

pub const DEFAULT_PG_PORT: u16 = 5432;

/// Starts a new Postgres 16 testcontainer
pub async fn start_pg_container() -> ContainerAsync<Postgres> {
    Postgres::default().with_tag("16").start().await.unwrap()
}

/// Creates new PgPool, connects to the testcontainer running on given host port, runs DB migrations.
/// Returns created pool
pub async fn create_pg_pool(port: u16) -> PgPool {
    let connection_string = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = PgPoolOptions::new()
        .max_connections(1) // https://github.com/launchbadge/sqlx/issues/2567
        .connect(connection_string.as_str())
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    pool
}
