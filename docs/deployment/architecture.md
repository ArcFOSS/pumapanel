# Deployment Architecture

## The two images

| Service | Dockerfile | Image | Listens | Published |
| --- | --- | --- | --- | --- |
| `api` | `apps/api/Dockerfile` | `pumapanel/api:latest` | `0.0.0.0:3001` | container network only |
| `web` | `apps/web/Dockerfile` | `pumapanel/web:latest` | `0.0.0.0:3000` | `127.0.0.1:3000` loopback |

Both are **multi-stage builds** from the repo root context:

- `api`: `rust:1.99-slim` → build with cached dependencies → copy the release binary into `debian:bookworm-slim`, run as **uid 10001**.
- `web`: `node:24.21.0-bookworm-slim` → install + `next build` → copy the **standalone server** kit into a fresh image, run as the **`node`** user.

Because `next.config.ts` sets `output: "standalone"`, the web image carries only what the server needs at runtime — a few megabytes instead of a full `node_modules`.

## Networking

`compose.yaml` creates a `pumapanel` bridge network:

- `web` and `api` resolve each other by **Compose service DNS** (`http://web:3000`, `http://api:3001`) — no hardcoded container IPs.
- `api` is only `expose`d on the network. The web app is the only public entry point (on loopback), and the reverse proxies are the only public ports.
- The API reads `HOST=0.0.0.0`, `PORT=3001` from its container environment — the same variables that default to loopback `127.0.0.1:3001` in local dev.

## Health checks

Declared **twice by design**:

1. **In each Dockerfile** (`HEALTHCHECK`) — used when the image is built with Docker, or with Podman using `BUILDAH_FORMAT=docker`.
2. **In `compose.yaml`** — required for Podman's default OCI images, which discard the Dockerfile `HEALTHCHECK`. Compose-level checks also feed `depends_on: condition: service_healthy` (used by the proxies).

The proxies wait for both backends to be `healthy` before starting, which avoids a proxy booting before its upstreams exist.

## Compose profiles

```
services:
  api:      # always
  web:      # always
  traefik:  # profiles: ["traefik"]
  nginx:    # profiles: ["nginx"]
```

Enable at most one profile — they bind the same host ports (`HTTP_PORT` / `HTTPS_PORT`):

```bash
docker compose up -d --build                 # web + api
docker compose --profile traefik up -d       # + Traefik :80/:443
docker compose --profile nginx   up -d       # + nginx :80
```

Traefik's configuration is split into:

- `deploy/traefik/traefik.yml` — **static** config: entrypoints, the HTTP→HTTPS redirect, and the **ACME resolver**. (Traefik cannot read resolver settings from CLI/env when `--configFile` is used, so they live in this file.)
- `deploy/traefik/dynamic.yml` — **dynamic** config: routers/services for `web` and `api`, `/api` prefix stripping, security headers, and `{{ env "DOMAIN" }}` templating (works on Docker _and_ Podman).

nginx's config lives in `deploy/nginx/conf.d/pumapanel.conf` and is HTTP-only by default; see [TLS](./tls.md) for the TLS options.

## Runtime environment

| Service | Variables | Values |
| --- | --- | --- |
| `api` | `HOST`, `PORT` | `0.0.0.0`, `3001` |
| `web` | `NODE_ENV`, `HOSTNAME`, `PORT` | `production`, `0.0.0.0`, `3000` |
| `traefik` | `DOMAIN` | from `.env` (default `localhost`) |
| stacks | `HTTP_PORT`, `HTTPS_PORT`, `WEB_PORT` | from `.env` |

`.env` is git-ignored; `.env.example` documents every variable with safe defaults. Never commit secrets. See [Reference: configuration](../reference/configuration.md).
