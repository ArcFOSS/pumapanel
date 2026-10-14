# Testing

Every layer is covered by a fast, scriptable gate that CI also runs.

## Rust tests (`apps/api`)

```bash
moon run api:test        # cargo test
```

Tests live in `tests/` and drive the **real** router via `tower::ServiceExt::oneshot`, so no socket is bound:

- `tests/auth.rs` — registration (including duplicate usernames/emails and case-insensitive normalization, plus malformed usernames), login by email **and** by username (success, wrong password, unknown identifier), an identical `401` body for both credential failures, JWT issuance and validation, tampered/expired/malformed tokens, missing-token `401`, cookie attributes, and logout clearing.
- `tests/admin.rs` — access control (unauthenticated `401`, non-admin `403` before any handler logic), role exposure on `/auth/me`, viewing and editing users, duplicate username/email `409`s, validation `400`s, unknown-id `404`s, role changes taking effect for existing sessions, and the self-demotion guard.
- `tests/config.rs` — configuration rules: production requires `JWT_SECRET`, short secrets are rejected, development generates an ephemeral secret, and `COOKIE_SECURE` defaults/overrides.

Shared request/response helpers (registration, login, cookie parsing, `TestContext`) live in `tests/common/mod.rs`.

Each test builds an in-memory SQLite pool, applies the real migrations, and constructs the app with a `Config` loaded through `Config::load(get: impl Fn(&str) -> Option<String>)`. That closure keeps tests deterministic and avoids mutating process-global environment state. Prefer this style for new tests:

- Extract logic into `src/` functions; keep `main.rs` thin.
- Test HTTP behavior through the router, not by hand-rolling `Request`s where an extractor is involved.
- Keep tests deterministic: no real network, and fixed clock/secret inputs.

(`tower` and `http-body-util` are dev-dependencies for request tests.)

## Frontend (`apps/web`)

```bash
moon run web:test        # vitest run
```

Suites live in `src/__tests__/` (config in `vitest.config.mts`, setup in `vitest.setup.ts`) and use [Vitest](https://vitest.dev) + [Testing Library](https://testing-library.com) on `jsdom`:

- `api.test.ts` — the client sends same-origin credentials to `/api/...`, returns parsed users, and maps failures (validation `fields`, network errors, non-JSON bodies) to `ApiError`; the admin client hits `/api/admin/users/{id}` with `GET`/`PATCH`.
- `auth-form.test.tsx` — required/malformed input (including the required username) blocks submission; valid input calls `onSubmit`; login accepts an email or a username; server field errors and generic alerts render; the button disables while in flight; the minimum password length is enforced.
- `auth-provider.test.tsx` — the provider bootstraps the session on mount, treats `401` as anonymous, and flips state on login/logout.
- `validate.test.ts` — shared username/email validators (length, allowed characters, malformed emails, whitespace trimming).
- `admin-user-view.test.tsx` / `admin-user-edit-form.test.tsx` — the profile view (fields, role badge, edit link) and the editor (client-side validation, role-selector guard for the account's own profile, PATCH payload, success and server field-error states).
- `dashboard-view.test.tsx` — details rows (including the role) and the admin-only tools section; `route-guard.test.ts` — `/admin`, `/admin/servers` paths are protected like `/dashboard`.
- `admin-dashboard-view.test.tsx` — the admin hub links to the servers and account sections.
- `server-list.test.tsx` — the empty state and the per-server rows (name, status, creation date).
- `create-server-dialog.test.tsx` — the create modal renders its form with a disabled submit and a notice when open, renders nothing when closed, and asks to close on cancel.
- `admin-servers-view.test.tsx` — the header action, the `?action=create` modal round-trip (open and cancel) asserted through nuqs' `withNuqsTestingAdapter`.

Global `fetch` is stubbed with `vi.stubGlobal` (module mocking is disallowed by the shared lint rules). Other gates:

```bash
moon run web:typecheck   # next typegen + tsc --noEmit
moon run web:lint        # Ultracite check
moon run web:build       # production build (catches import/rendering errors)
```

## Container smoke tests

The fastest end-to-end validation of the whole stack:

```bash
cp .env.example .env      # then set JWT_SECRET (openssl rand -base64 48)
docker compose up -d --build
docker compose ps                        # both services should show (healthy)
curl -fsS http://127.0.0.1:3000/login | grep -qi pumapanel && echo web-ok
curl -fsS http://127.0.0.1:3001/health   # → {"status":"ok"}

# Exercise auth end to end through the web origin
curl -fsS -c /tmp/pp.jar -H 'content-type: application/json' \
  -d '{"username":"demo","email":"you@example.com","password":"password123"}' \
  http://127.0.0.1:3000/api/auth/register
curl -fsS -b /tmp/pp.jar http://127.0.0.1:3000/api/auth/me
# Sign in with either the email or the username
curl -fsS -H 'content-type: application/json' \
  -d '{"identifier":"demo","password":"password123"}' \
  http://127.0.0.1:3000/api/auth/login
docker compose down
```

For proxy validation see [Deployment → operations](../deployment/operations.md).

## What CI runs

- lint (Oxlint + clippy with `-D warnings`)
- type-check (TS + Rust)
- API tests (`moon run api:test`)
- web tests (`moon run web:test`)
- web production build
- Compose config validation (base + both proxy profiles)
- Container image builds

If it fails locally, it fails in CI — and vice versa.
