# Testing

Every layer is covered by a fast, scriptable gate that CI also runs.

## Rust tests (`apps/api`)

```bash
moon run api:test        # cargo test
```

Tests live in `tests/` and drive the **real** router via `tower::ServiceExt::oneshot`, so no socket is bound:

- `tests/auth.rs` — registration (including duplicate emails and case-insensitive normalization), login (success, wrong password, unknown user), an identical `401` body for both credential failures, JWT issuance and validation, tampered/expired/malformed tokens, missing-token `401`, cookie attributes, and logout clearing.
- `tests/config.rs` — configuration rules: production requires `JWT_SECRET`, short secrets are rejected, development generates an ephemeral secret, and `COOKIE_SECURE` defaults/overrides.

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

- `api.test.ts` — the client sends same-origin credentials to `/api/...`, returns parsed users, and maps failures (validation `fields`, network errors, non-JSON bodies) to `ApiError`.
- `auth-form.test.tsx` — required/malformed input blocks submission; valid input calls `onSubmit`; server field errors and generic alerts render; the button disables while in flight; the minimum password length is enforced.
- `auth-provider.test.tsx` — the provider bootstraps the session on mount, treats `401` as anonymous, and flips state on login/logout.

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
curl -fsS http://127.0.0.1:3000/ | grep -qi pumapanel && echo web-ok
curl -fsS http://127.0.0.1:3001/health   # → {"status":"ok"}

# Exercise auth end to end through the web origin
curl -fsS -c /tmp/pp.jar -H 'content-type: application/json' \
  -d '{"email":"you@example.com","password":"password123"}' \
  http://127.0.0.1:3000/api/auth/register
curl -fsS -b /tmp/pp.jar http://127.0.0.1:3000/api/auth/me
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
