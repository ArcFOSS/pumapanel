# Configuration Reference

Where configuration lives, what the variables mean, and which defaults apply.

## Toolchain (`.moon/toolchains.yml`)

| Key | Value | Notes |
| --- | --- | --- |
| `node.version` | `24.21.0` | Installed via proto |
| `pnpm.version` | `12.12.0` | Also pinned in root `package.json` (`packageManager`) |
| `rust.version` | `1.99.0` | Nightly-level tooling not required |

## Environment (`.env` — git-ignored, template in `.env.example`)

| Variable | Default | Used by | Purpose |
| --- | --- | --- | --- |
| `DOMAIN` | `example.com` | Traefik profile | Public hostname for routing + Let's Encrypt |
| `HTTP_PORT` | `80` | Proxy profiles + `compose.yaml` | Host port for HTTP (`< 1024` needs rootful Docker or Podman workaround) |
| `HTTPS_PORT` | `443` | Traefik profile | Host port for HTTPS |
| `WEB_PORT` | `3000` | `web` service | Loopback port for direct access |
| `JWT_SECRET` | — | `api` service | Signing key for session JWTs. **Required** in production, ≥32 chars. Generate with `openssl rand -base64 48` |
| `JWT_EXPIRATION_SECS` | `86400` | `api` service | Token/cookie lifetime in seconds |
| `COOKIE_SECURE` | `false` in Compose | `api` service | Adds `Secure` to the session cookie. Set `true` when served over HTTPS (Traefik or TLS upstream of nginx) |
| `DATABASE_URL` | `sqlite://pumapanel.db` | `api` (local dev) | SQLite file, created next to the API crate on first run. Compose overrides it with a path on the `api-data` volume |
| `API_URL` | `http://127.0.0.1:3001` | `web` (build-time) | Target of the `/api/*` rewrite; baked into the web build |
| `AGENT_URL` | unset in Compose | `api` | Base URL of the container-management agent. When unset the panel disables server management. Local dev uses `http://127.0.0.1:9091` |
| `AGENT_TOKEN` | — | `api`, `agent` | Shared secret (`>= 16` characters) for API <-> agent requests. The agent refuses to start without it; the API requires it whenever `AGENT_URL` is set |

> The API and the agent load this file at startup with [`dotenvy`](https://crates.io/crates/dotenvy), searching the current directory and its parents, so `moon run api:dev`, `moon run agent:dev`, and Docker Compose share one `.env`. Already-set environment variables always take precedence, and the file is optional (containers inject configuration directly).

> The API validates `JWT_SECRET` at startup. With `APP_ENV=production` (the Compose default) and no secret it exits with a clear error instead of falling back to an insecure value, so `docker compose up` **requires** `JWT_SECRET` in `.env`.

## Runtime environment variables (containers)

| Service | Variable | Value | Purpose |
| --- | --- | --- | --- |
| `api` | `HOST` | `0.0.0.0` | Bind in the container (local default: `127.0.0.1`) |
| `api` | `PORT` | `3001` | Listen port |
| `api` | `APP_ENV` | `production` | Enables production config rules (e.g. required `JWT_SECRET`) |
| `api` | `DATABASE_URL` | `sqlite:///app/data/pumapanel.db` | SQLite database on the `api-data` volume |
| `web` | `NODE_ENV` | `production` | Next.js production mode |
| `web` | `HOSTNAME` | `0.0.0.0` | Next standalone server bind address |
| `web` | `PORT` | `3000` | Next standalone server port |

`API_URL` is a **build-time** argument for the `web` image (default `http://127.0.0.1:3001`); Compose passes `http://api:3001` so the `/api/*` rewrite reaches the API over the Compose network. See [Web architecture](../architecture/web.md).

## Persistence

The API stores accounts in SQLite. `compose.yaml` mounts the named volume `api-data` at `/app/data`, so users survive image rebuilds. Migrations are embedded in the binary and applied automatically on startup. Remove the database with `docker compose down -v`.

## Compose (`compose.yaml`)

- `pumapanel` bridge network; services reach each other by name (`web:3000`, `api:3001`).
- `api` is `expose`d (internal) only; `web` publishes `127.0.0.1:${WEB_PORT:-3000}:3000`.
- `traefik`/`nginx` are profile-gated; both use `HTTP_PORT` / `HTTPS_PORT`.
- Health checks are defined both in the Dockerfiles and in `compose.yaml` for Podman-OCI compatibility.

## Reverse proxies

| File | Content |
| --- | --- |
| `deploy/traefik/traefik.yml` | Static config: entrypoints, HTTP→HTTPS redirect, ACME resolver |
| `deploy/traefik/dynamic.yml` | Routers/services, `/api` prefix strip, security headers, `{{ env "DOMAIN" }}` |
| `deploy/nginx/conf.d/pumapanel.conf` | HTTP proxy to `web:3000`/`api:3001`, commented TLS block |

> Traefik's ACME resolver **cannot** be set via CLI/env when the static config is loaded with `--configFile` — edit `traefik.yml` directly.

## Lint / format

| File | Purpose |
| --- | --- |
| `oxlint.config.mts` / `oxfmt.config.mts` (root) | Re-export `@pumapanel/lint` |
| `packages/lint/oxlint.config.mjs` | Shared rules incl. shadcn `components/ui/**` overrides |
| `packages/lint/oxfmt.config.mjs` | Shared formatting |
| `apps/web/oxlint.config.mts` / `oxfmt.config.mts` | Project-level re-exports |

Run `pnpm exec ultracite doctor` to validate the setup.

## Git/hooks/CI

| File                            | Purpose                               |
| ------------------------------- | ------------------------------------- |
| `.husky/pre-commit`             | Runs `ultracite fix` on staged files  |
| `.github/hooks/ultracite.json`  | Agent/editor post-tool-use formatting |
| `.github/workflows/ci.yml`      | Quality + container validation        |
| `.github/workflows/release.yml` | GHCR images + GitHub release on tags  |
| `.github/dependabot.yml`        | Dependency update cadences            |

## See also

- [Ports](./ports.md) for the complete network map.
- [Development setup](../development/setup.md) for the local toolchain.
