CREATE TABLE IF NOT EXISTS users
(
    id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username text UNIQUE NOT NULL,
    password text        NOT NULL,
    email    text UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS event_type
(
    id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid   NOT NULL REFERENCES users (id),
    name    text   NOT NULL,
    tags    text[] NOT NULL  DEFAULT array []::text[],
    UNIQUE (user_id, name)
);

CREATE TABLE IF NOT EXISTS journal_entry
(
    id            uuid PRIMARY KEY     DEFAULT gen_random_uuid(),
    user_id       uuid        NOT NULL REFERENCES users (id),
    event_type_id uuid        NOT NULL REFERENCES event_type (id) ON DELETE CASCADE,
    description   text,
    tags          text[]      NOT NULL DEFAULT array []::text[],
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_journal_entry_user_event_type on journal_entry (user_id, event_type_id);
