# API (`apps/api`)

The backend is a Rust HTTP service built with [axum](https://github.com/tokio-rs/axum) 0.8 on [tokio](https://tokio.rs), backed by SQLite via [sqlx](https://github.com/launchbadge/sqlx). It serves the health probe and an email/username/password authentication API.

## Routes

| Method | Path | Auth | Success | Purpose |
| --- | --- | --- | --- | --- |
| `GET` | `/health` | — | `200 {"status":"ok"}` | Liveness probe |
| `POST` | `/auth/register` | — | `201 {"user":{…}}` | Create an account, hash the password, sign in |
| `POST` | `/auth/login` | — | `200 {"user":{…}}` | Verify credentials, sign in |
| `POST` | `/auth/logout` | — | `204` | Clear the session cookie |
| `GET` | `/auth/me` | ✅ | `200 {"user":{…}}` | Return the current user |
| `GET` | `/admin/users/{id}` | ✅ admin | `200 {"user":{…}}` | View any user's profile |
| `PATCH` | `/admin/users/{id}` | ✅ admin | `200 {"user":{…}}` | Edit username, email, or role |

`PublicUser` is `{ "id", "email", "username", "role", "createdAt" }` (`camelCase` on the wire, `role` is `"user"` or `"admin"`). The stored `password_hash` is never serialized.

In the containerized deployment the reverse proxy serves these under `/api` (stripping the prefix); during `next dev` the web app rewrites `/api/*` to the API. The browser therefore only ever talks to them **same-origin** — see [Web app](./web.md).

## Authentication

The flow is a stateless JWT stored in an **HttpOnly cookie**; there are no server-side sessions.

- **Registration** normalizes the username and email (`trim` + lowercase), validates them (the username must be 3–30 characters of letters, numbers, or underscores), hashes the password with **Argon2id** (random per-password salt), inserts the user with `INSERT … RETURNING`, issues a token, and sets the cookie. Duplicate usernames or emails are rejected by the database `UNIQUE` constraints and surfaced as `409` (`username_taken` / `email_taken`).
- **Login** looks the user up by the submitted identifier — an email address or a username — and verifies the hash. Unknown accounts and wrong passwords both return an identical generic `401`; unknown accounts additionally run a dummy hash verification so the two paths take comparable time.
- **Logout** clears the cookie. Because JWTs are stateless, a previously issued token stays valid until it expires (documented limitation).
- **`/auth/me`** is guarded by the `AuthUser` extractor, which reads the token from the session cookie (falling back to `Authorization: Bearer`), verifies it, then loads the user by id. Every failure maps to `401`.
- **Roles.** Every account has a `role` (`user` or `admin`) read fresh from the database on **each** request — the token only carries `sub`/`exp` — so a role change takes effect immediately, including for already-issued tokens. `PublicUser.role` lets the client adapt the UI, but the API is the enforcement point.
- **Admin routes** (`src/admin.rs`) sit behind the `AdminUser` extractor, which first resolves `AuthUser` and then requires `role == admin`, rejecting anything else with `403` _before_ handler logic runs — unauthorized callers learn nothing about the target account. The `role` payload field deserializes via `#[serde(rename_all = "lowercase")]`, so a database that somehow receives a bad value fails the `CHECK` constraint.
- **Admin edits.** `PATCH /admin/users/{id}` accepts any subset of `{ username, email, role }`, validates provided fields with the same rules as registration (3–30 alphanumeric/underscore username, valid email), and maps duplicate-username/email conflicts to the same `409` codes. An administrator may change anyone's role, but **never their own** — that is the route by which a lone admin could demote themselves and lock everyone out.

### Tokens (`src/auth/jwt.rs`)

- **Algorithm** HS256, pinned on decode via a `jsonwebtoken::Validation` that also requires the issuer and the `exp`/`sub` claims.
- **Claims** `{ sub, iss, iat, exp }`; timestamps are UTC epoch seconds.
- **Secret** comes from `JWT_SECRET` (see [Configuration](#configuration)); it is validated at startup and never logged.
- **Lifetime** is `JWT_EXPIRATION_SECS` (default 24h). Expired or tampered tokens fail verification and yield `401`.

### Cookies (`src/auth/mod.rs`)

| Attribute  | Value                                                          |
| ---------- | -------------------------------------------------------------- |
| Name       | `COOKIE_NAME` (default `pumapanel_token`)                      |
| `HttpOnly` | always                                                         |
| `SameSite` | `Lax`                                                          |
| `Secure`   | `COOKIE_SECURE` (defaults to `true` when `APP_ENV=production`) |
| `Path`     | `/`                                                            |
| `Max-Age`  | the token lifetime; cleared on logout                          |

## Persistence

- **SQLite** through `sqlx` (bundled, so no system SQLite needed). `DATABASE_URL` defaults to `sqlite://pumapanel.db`; the container uses `sqlite:///app/data/pumapanel.db` on a named volume.
- **Migrations** live in `apps/api/migrations/` and are embedded into the binary with `sqlx::migrate!("./migrations")`, so `init_pool` applies them automatically on startup — no separate migrate step.
- The pool enables `create_if_missing`, foreign keys, and a 5s busy timeout.
- The `users` table stores `id` (UUIDv4 text), `username` (`UNIQUE`), `email` (`UNIQUE`), `password_hash`, `role` (`CHECK role IN ('user','admin')`, default `'user'`), and `created_at`. Usernames and emails are stored lowercased, so uniqueness is effectively case-insensitive.
- **Roles are migrated, not hardcoded.** `0003_add_role.sql` adds the column, and `0004_promote_initial_admin.sql` is an idempotent data migration keyed by user id that promotes exactly one bootstrap account to `admin`. No application code contains privileged ids; a fresh deployment simply has no admins until one is promoted.

## Error format

Every error response is JSON with a stable shape:

```json
{
  "error": {
    "code": "validation_error",
    "message": "Validation failed",
    "fields": [{ "field": "email", "message": "Enter a valid email address" }]
  }
}
```

| `code` | Status | Meaning |
| --- | --- | --- |
| `validation_error` | `400` | Input failed validation (`fields` lists offenders) |
| `invalid_credentials` | `401` | Wrong email, username, or password (generic) |
| `unauthorized` | `401` | Missing/invalid/expired token |
| `forbidden` | `403` | Authenticated, but the role is insufficient |
| `not_found` | `404` | The requested user/resource does not exist |
| `email_taken` | `409` | Email already registered |
| `username_taken` | `409` | Username already taken |
| `internal_error` | `500` | Unexpected failure (detail logged, never returned) |

## Configuration

Loaded once at startup from the environment (`src/config.rs`). Invalid or missing values fail fast with a message that is safe to print.

| Variable | Default | Notes |
| --- | --- | --- |
| `HOST` / `PORT` | `127.0.0.1` / `3001` | Containers set `HOST=0.0.0.0` |
| `DATABASE_URL` | `sqlite://pumapanel.db` | sqlx SQLite URL |
| `APP_ENV` | development | `production`/`prod` enables production rules |
| `JWT_SECRET` | — | **Required in production**; ≥32 chars; dev generates an ephemeral secret |
| `JWT_EXPIRATION_SECS` | `86400` | Positive integer |
| `JWT_ISSUER` | `pumapanel` | `iss` claim, validated on decode |
| `COOKIE_NAME` | `pumapanel_token` |  |
| `COOKIE_SECURE` | `APP_ENV == production` | Force `true`/`false` to override |
| `COOKIE_DOMAIN` | — | Set only when sharing across subdomains |

See [Reference: configuration](../reference/configuration.md) for the full list.

## Behavior

`apps/api/src/main.rs` is a thin binary; the router lives in `src/lib.rs` so integration tests drive the real app without binding a socket.

- **Binding.** `HOST`/`PORT`, defaulting to `127.0.0.1:3001`.
- **Graceful shutdown.** `CTRL_C` and `SIGTERM` are awaited via `tokio::select!`; in-flight requests finish before exit.

## Dependencies (`Cargo.toml`)

| Crate | Version | Purpose |
| --- | --- | --- |
| `axum` | 0.8 | HTTP framework |
| `axum-extra` | 0.10 | Typed cookie extraction (`CookieJar`) |
| `cookie` | 0.18 | Cookie attributes (`Max-Age` durations) |
| `tokio` | 1.x | async runtime (`macros`, `rt-multi-thread`, `signal`) |
| `serde` / `serde_json` | 1.x | JSON serialization |
| `sqlx` | 0.8 | SQLite pool, queries, embedded migrations (`runtime-tokio`, `sqlite`, `migrate`, bundled) |
| `argon2` | 0.5 | Argon2id password hashing |
| `jsonwebtoken` | 9.x | JWT signing/verification |
| `validator` | 0.20 | Declarative request validation |
| `uuid` | 1.x | User ids and ephemeral dev secrets |
| `rayon` | 1.x | data parallelism (reserved for future workloads) |

Edition `2024` with the Rust 1.99 toolchain from `.moon/toolchains.yml`. `tower` is a dev-dependency for request tests.

## Quality gates

- `moon run api:test` — `cargo test` (config, auth, and admin integration tests in `tests/`; shared helpers in `tests/common/`).
- `moon run api:lint` — `cargo clippy --all-targets --all-features -- -D warnings`.
- `moon run api:typecheck` — `cargo check --all-targets`.
- `moon run api:build` — `cargo build --release`.

## Scope and limitations

In scope: registration, login, logout, session inspection, JWT lifecycle, persistence, and role-based authorization (admin user management with backend-enforced access control). Out of scope by design: OAuth/social login, email verification and password reset, MFA, refresh-token rotation, and user self-service account editing. Logout is cookie-only (tokens are not revoked server-side).

## See also

- [Web app](./web.md) — how the frontend consumes these routes.
- [Deployment architecture](../deployment/architecture.md) — packaging and the `HOST`/`PORT` contract with Compose.
- [Reference: ports](../reference/ports.md) — where the API listens.
