# journal-backend

Backend for simple journaling system written in Rust.

Journal allows users to:
* Register new user account and log-in into the application.
* Create and manage event types that can be used in the journal entries.
* Optionally assign tags for each event type.
* _(work in progress)_ Log journal entries with event types and tags.
* _(work in progress)_ Search entries using various criteria.

## Running the project locally

The only prerequisite is to have PostgreSQL 13 or later available locally (e.g. via docker container).

### Configuration

Configuration is loaded by reading environment variables from the `.env` file.
The most important environment variable is `DATABASE_URL` which must point to a valid Postgres database.
If such a database doesn't exist, please create it before running the project.

**Tip**: You can easily create database referenced in `.env` file using [sqlx-cli](https://crates.io/crates/sqlx-cli)
by executing following command from the root directory of the project:

```
sqlx database create
```

**Database migrations** can be executed either during application startup, by setting `DB_MIGRATE_ON_START` to `true`,
or using [sqlx-cli](https://crates.io/crates/sqlx-cli) tool by executing following command from the root directory of the project:

```
sqlx migrate run
```

### Starting the application

To start the application, just execute `cargo run` in the project's root directory.

### Running the tests

To run all tests, just execute `cargo test` in the project's root directory.