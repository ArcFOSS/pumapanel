-- Add an authorization role to user accounts.
--
-- New accounts default to `user`; only an administrator (or a data migration)
-- promotes an account to `admin`. The CHECK constraint mirrors the `Role` enum
-- in Rust, so an invalid value also fails at the database boundary.
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin'));