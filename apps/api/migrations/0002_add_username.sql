-- Add a required, unique username to user accounts.
--
-- SQLite cannot add a `NOT NULL` column to a populated table in place, so the
-- table is rebuilt. Existing rows (accounts created before usernames existed)
-- keep working: their username is derived from the email local part, and a
-- numeric suffix resolves any collisions. New usernames are validated and
-- normalized by the application, which also stores them lowercased so the
-- UNIQUE constraint is effectively case-insensitive.
CREATE TABLE users_new (
    id            TEXT NOT NULL PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

WITH derived AS (
    SELECT
        id,
        email,
        password_hash,
        created_at,
        lower(substr(email, 1, instr(email, '@') - 1)) AS base,
        row_number() OVER (
            PARTITION BY lower(substr(email, 1, instr(email, '@') - 1))
            ORDER BY created_at, id
        ) AS duplicate_index
    FROM users
)
INSERT INTO users_new (id, email, username, password_hash, created_at)
SELECT
    id,
    email,
    CASE
        WHEN base = '' THEN 'user_' || substr(id, 1, 8)
        WHEN duplicate_index = 1 THEN base
        ELSE base || '_' || duplicate_index
    END,
    password_hash,
    created_at
FROM derived;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
