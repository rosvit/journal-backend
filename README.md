# journal-backend

Backend for simple journaling system written in Rust.

Journal allows users to:
* Register new user account and log-in into the application.
* Create and manage event types that can be used in the journal entries.
* Optionally assign tags for each event type.
* Log journal entries with event types and tags.
* Search entries using various criteria.

## Running the project locally

The only prerequisite is to have PostgreSQL 13 or later available locally (e.g. via docker container).

### Compilation

This project uses the `sqlx` library which performs SQL query validation during compile time against a running database
with up-to-date schema (all migrations applied). See [Configuration](#Configuration) for more details on how to
execute the database migrations.

:information_source: To skip the SQL validation, set the environment variable `SQLX_OFFLINE=true`, or add it to the
`.env` file.

### Configuration

Configuration is loaded by reading environment variables from the `.env` file.
The most important environment variable is `DATABASE_URL` which must point to a valid Postgres database.
If such a database doesn't exist, please create it before running the project.

**Tip**: You can easily create database referenced in `.env` file using [sqlx-cli](https://crates.io/crates/sqlx-cli)
by executing following command from the root directory of the project:

```shell
sqlx database create
```

**Database migrations** can be executed either during application startup, by setting `DB_MIGRATE_ON_START` to `true`
(this requires also `SQLX_OFFLINE=true` if the project hasn't been yet compiled),
or using [sqlx-cli](https://crates.io/crates/sqlx-cli) tool by executing following command from the root directory of the project:

```shell
sqlx migrate run
```

#### Other configuration variables

All variables in the `.env` file can be modified. It is also possible to override them by simply setting the
corresponding environment variable to the required value.

### Starting the application

To start the application, just execute `cargo run` in the project's root directory.

### Running the tests

To run all tests, just execute `cargo test` in the project's root directory.

### Running in Docker

1. Build Docker image:

    ```shell
    docker build -t journal-backend:latest . 
    ```

2. Run the application:

   :warning: Pass correct `DATABASE_URL`, pointing to the running Postgres with an existing database with up-to-date
schema, or enable DB migrations on startup (see [Configuration](#Configuration) for more details).
    ```shell
   docker run -p 8080:8080 -e DATABASE_URL=postgres://postgres:pgpass@192.168.106.2/journal -d journal-backend:latest
   ```
