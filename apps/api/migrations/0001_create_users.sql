-- User accounts for authentication.
--
-- `email` is stored lowercased/trimmed by the application, and the UNIQUE
-- constraint (plus the database-level unique index) guarantees that concurrent
-- registration attempts cannot create duplicate accounts.
CREATE TABLE IF NOT EXISTS users (
    id            TEXT NOT NULL PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
