use log::debug;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use std::future::Future;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::ImageExt;
use tokio::runtime;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

pub const DEFAULT_PG_PORT: u16 = 5432;

#[derive(Debug)]
pub enum ContainerCommand {
    GetPort,
    Stop,
}

pub struct Channel<T> {
    pub tx: Sender<T>,
    pub rx: Mutex<Receiver<T>>,
}

/// Creates channels for communication with shared Postgres testcontainer
pub fn channel<T>() -> Channel<T> {
    let (tx, rx) = mpsc::channel(32);
    Channel { tx, rx: Mutex::new(rx) }
}

/// Executes given Future on the calling thread and blocks it until the future completes
pub fn execute_blocking<F: Future>(f: F) {
    runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f);
}

/// Starts a new Postgres 16 testcontainer and waits for incoming container commands
pub async fn start_pg_container(
    input_chan: &Channel<ContainerCommand>,
    pg_chan: &Channel<u16>,
    stop_chan: &Channel<()>,
) {
    let container = Postgres::default().with_tag("16").start().await.unwrap();
    let port = container.get_host_port_ipv4(DEFAULT_PG_PORT).await.unwrap();
    debug!("Postgres container started on port {}", port);

    let mut rx = input_chan.rx.lock().await;
    while let Some(command) = rx.recv().await {
        debug!("Received container command: {:?}", command);
        match command {
            ContainerCommand::GetPort => pg_chan.tx.send(port).await.unwrap(),
            ContainerCommand::Stop => {
                container.stop().await.unwrap();
                container.rm().await.unwrap();
                stop_chan.tx.send(()).await.unwrap();
                rx.close();
                break;
            }
        }
    }
}

/// Creates new PgPool, connects to the testcontainer running on given host port, runs DB migrations.
/// Returns created pool
pub async fn create_pg_pool(port: u16) -> PgPool {
    let db_url = format!("postgres://postgres:postgres@127.0.0.1:{port}");
    let db_name = Uuid::new_v4().to_string();
    let connection_string = format!("{db_url}/{db_name}");

    // Create test database
    let db_pool = PgPoolOptions::new().max_connections(1).connect(&db_url).await.unwrap();
    db_pool.execute(format!(r#"CREATE DATABASE "{}";"#, &db_name).as_str()).await.unwrap();
    db_pool.close().await;

    // Create actual pool for the test case and run DB migrations
    let pool = PgPoolOptions::new()
        .max_connections(1) // https://github.com/launchbadge/sqlx/issues/2567
        .connect(&connection_string)
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    pool
}

/// Gets the actual port on host machine for shared Postgres testcontainer
pub async fn get_pg_port(input_chan: &Channel<ContainerCommand>, pg_chan: &Channel<u16>) -> u16 {
    input_chan.tx.send(ContainerCommand::GetPort).await.unwrap();
    pg_chan.rx.lock().await.recv().await.unwrap()
}

/// Performs clean-up after tests (stops and removes shared Postgres testcontainer)
pub fn clean_up(input_chan: &Channel<ContainerCommand>, stop_chan: &Channel<()>) {
    input_chan.tx.blocking_send(ContainerCommand::Stop).unwrap();
    stop_chan.rx.blocking_lock().blocking_recv().unwrap();
}
