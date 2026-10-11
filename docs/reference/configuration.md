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

## Runtime environment variables (containers)

| Service | Variable | Value | Purpose |
| --- | --- | --- | --- |
| `api` | `HOST` | `0.0.0.0` | Bind in the container (local default: `127.0.0.1`) |
| `api` | `PORT` | `3001` | Listen port |
| `web` | `NODE_ENV` | `production` | Next.js production mode |
| `web` | `HOSTNAME` | `0.0.0.0` | Next standalone server bind address |
| `web` | `PORT` | `3000` | Next standalone server port |

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
