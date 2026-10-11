# Architecture Overview

PumaPanel is a two-service system with a small amount of infrastructure around it:

- **`web`** — a Next.js 16 / React 19 frontend served by Next's standalone server.
- **`api`** — a Rust service built with axum + tokio.
- **Proxies** — optionally Traefik or nginx in front of both, for TLS and routing.
- **Tooling** — moon (task orchestration + toolchains), pnpm (JS/TS workspace), and Ultracite (lint/format) sit _around_ the apps but do not run in production.

```
                    ┌──────────────────────────┐
   browser ────────▶│  reverse proxy (opt.)    │  Traefik :80/:443
                    │  Traefik  or  nginx      │  nginx   :80
                    └────────────┬─────────────┘
                                 │
                    ┌────────────▼─────────────┐
                    │        web  :3000        │  Next.js standalone
                    │  (published on loopback) │
                    └────────────┬─────────────┘
                                 │  /api/*  (prefix stripped)
                    ┌────────────▼─────────────┐
                    │        api  :3001        │  axum + tokio
                    │  (container network      │
                    │   only — not published)  │
                    └──────────────────────────┘
```

## Request paths

| Scenario | URL | Served by |
| --- | --- | --- |
| Local dev, no proxy | `http://127.0.0.1:3000` | `web` (Next dev server) |
| Local dev, direct API | `http://127.0.0.1:3001/health` | `api` |
| Containerized, no proxy | `http://127.0.0.1:3000` | `web` (published on loopback) |
| Behind Traefik | `https://example.com/` and `/api/*` | `web` / `api` (Traefik routes) |
| Behind nginx | `http://example.com/` and `/api/*` | `web` / `api` (nginx routes) |

Behind a proxy the `/api/*` prefix is stripped, so `/api/health` reaches the API as `/health`.

## Design decisions

- **One repo, two apps.** The frontend and backend live in `apps/` and share a root workspace, so a single `git` history and one CI pipeline cover the whole product. See [Monorepo and toolchain](./monorepo.md).
- **Separation of concerns.** The API only exposes `/health` today; the route table and the `HOST`/`PORT` environment handling are designed to grow without reworking the container setup.
- **Proxy-optional.** The stack runs without a proxy (`web` is reachable directly on the loopback), but ships with two ready-made proxy profiles that do not conflict with each other. See [Deployment](../deployment/README.md).
- **Containers are the deployable unit.** Both apps build into slim, non-root OCI images; health checks are declared in both the Dockerfiles and `compose.yaml` so Docker _and_ Podman honor them. See [Deployment architecture](../deployment/architecture.md).
- **Pinned toolchains.** Node, pnpm, and Rust versions are pinned in `.moon/toolchains.yml` and enforced everywhere (local, CI, containers) via proto/moon. See [Monorepo and toolchain](./monorepo.md).

## Next steps

- Understand the orchestration: [Monorepo and toolchain](./monorepo.md)
- Read the frontend: [Web app](./web.md)
- Read the backend: [API](./api.md)
