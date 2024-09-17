use actix_cors::Cors;
use actix_web::middleware::{from_fn, Logger};
use actix_web::{web, App, HttpServer};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};
use dotenv::dotenv;
use env_logger::Env;
use journal_backend::model::Config;
use journal_backend::user::handler::*;
use journal_backend::user::middleware::*;
use journal_backend::user::repository::PostgresUserRepository;
use journal_backend::user::service::UserServiceImpl;
use log::debug;
use sqlx::PgPool;
use std::env;
use std::time::Duration;

const ROOT: &str = "";
type DefaultUserService = UserServiceImpl<PostgresUserRepository>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let config = load_app_config();
    let metrics = setup_metrics();
    let pool = PgPool::connect(&config.database_url).await.unwrap();
    migrate_db(&pool, config.db_migrate_on_start).await;
    let user_repository = PostgresUserRepository::new(pool.clone());
    let user_service = web::Data::new(UserServiceImpl::new(
        user_repository,
        config.jwt_encoding_key_secret.clone(),
        config.jwt_exp_duration,
    ));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::permissive())
            .wrap(metrics.clone())
            .app_data(user_service.clone())
            .service(
                web::scope("/user")
                    .route(ROOT, web::post().to(register::<DefaultUserService>))
                    .route("/login", web::post().to(login::<DefaultUserService>))
                    .service(
                        web::resource("/{user_id}")
                            .wrap(from_fn(validate_caller_id))
                            .wrap(HttpAuthentication::bearer(
                                access_token_validator::<DefaultUserService>,
                            ))
                            .put(update_password::<DefaultUserService>),
                    ),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

fn load_app_config() -> Config {
    let db_url = env::var("DATABASE_URL").expect("Could not find DATABASE_URL env. variable");
    let db_migrate = env::var("DB_MIGRATE_ON_START")
        .expect("Could not find DATABASE_URL env. variable")
        .parse::<bool>()
        .expect("Could not convert string value of DB_MIGRATE_ON_START to bool");
    let jwt_secret = env::var("JWT_ENCODING_KEY_SECRET")
        .expect("Could not find JWT_ENCODING_KEY_SECRET env. variable");
    let jwt_exp_secs = env::var("JWT_EXPIRATION_SECS")
        .expect("Could not find JWT_EXPIRATION_SECS env. variable")
        .parse::<u64>()
        .expect("Could not convert string value of JWT_EXPIRATION_SECS to u64");

    Config {
        database_url: db_url,
        db_migrate_on_start: db_migrate,
        jwt_encoding_key_secret: jwt_secret,
        jwt_exp_duration: Duration::from_secs(jwt_exp_secs),
    }
}

async fn migrate_db(pool: &PgPool, should_run: bool) {
    if should_run {
        debug!("Running DB migrations");
        sqlx::migrate!().run(pool).await.unwrap()
    } else {
        debug!("Skipping DB migrations")
    }
}

fn setup_metrics() -> PrometheusMetrics {
    let metrics_path = "/metrics";
    PrometheusMetricsBuilder::new("server")
        .endpoint(metrics_path)
        .exclude(metrics_path)
        .build()
        .unwrap()
}
