<div align="center">

# PumaPanel

A production-minded template monorepo: a **Next.js web frontend** + a **Rust API backend**, orchestrated with **moon** and **pnpm** — deployable with **Docker** or **Podman** behind **Traefik** or **nginx**.

![CI status](https://shieldcn.dev/github/ci/ArcFOSS/pumapanel.svg) ![License](https://shieldcn.dev/github/license/ArcFOSS/pumapanel.svg) ![GitHub stars](https://shieldcn.dev/github/stars/ArcFOSS/pumapanel.svg) ![Forks](https://shieldcn.dev/github/forks/ArcFOSS/pumapanel.svg) ![Last commit](https://shieldcn.dev/github/last-commit/ArcFOSS/pumapanel.svg) ![Open issues](https://shieldcn.dev/github/open-issues/ArcFOSS/pumapanel.svg) ![Open PRs](https://shieldcn.dev/github/open-prs/ArcFOSS/pumapanel.svg) ![Contributors](https://shieldcn.dev/github/contributors/ArcFOSS/pumapanel.svg)

![Next.js](https://shieldcn.dev/badge/Next.js-16-000000.svg?logo=next.js&logoColor=white) ![React](https://shieldcn.dev/badge/React-19-61DAFB.svg?logo=react&logoColor=white) ![Tailwind CSS](https://shieldcn.dev/badge/Tailwind_CSS-4-06B6D4.svg?logo=tailwindcss&logoColor=white) ![Node](https://shieldcn.dev/badge/Node-24.21.0-339933.svg?logo=node.js&logoColor=white) ![pnpm](https://shieldcn.dev/badge/pnpm-12.12.0-F69220.svg?logo=pnpm&logoColor=white) ![Rust](https://shieldcn.dev/badge/Rust-1.99.0-000000.svg?logo=rust&logoColor=white) ![Docker](https://shieldcn.dev/badge/Docker-ready-2496ED.svg?logo=docker&logoColor=white) ![PRs welcome](https://shieldcn.dev/badge/PRs-welcome-brightgreen.svg)

</div>

## What is PumaPanel?

PumaPanel is a monorepo that demonstrates and standardizes a modern full-stack setup: a **React 19 / Next.js 16** frontend styled with **Tailwind CSS v4** and **shadcn/ui**, a **Rust (axum + tokio)** API backend, and a container-first deployment story that works across **Docker and Podman** with your choice of reverse proxy. Everything is version-pinned and enforced through [moon](https://moonrepo.dev) tasks and **Ultracite** linting/formatting.

## Features

- **Next.js 16** app router frontend with React Compiler, standalone output, and Tailwind CSS v4
- **Rust API** built with axum + tokio — graceful shutdown, `HOST`/`PORT` config
- **Email/password authentication**: Argon2id hashing, JWT sessions in an HttpOnly cookie, SQLite persistence via sqlx
- **Containerized from day one**: slim, non-root OCI images with health checks
- **Docker _and_ Podman** via the same `compose.yaml`
- **Traefik** (automatic Let's Encrypt TLS) or **nginx** as mutually-exclusive proxy profiles
- **CI/CD**: GitHub Actions pipeline (`lint → typecheck → test → build → container checks`) and GHCR image releases
- **Strict quality gates**: Ultracite (Oxlint + Oxfmt), `cargo clippy`, pinned toolchains

## Repository layout

| Path | Description |
| --- | --- |
| `apps/web` | Next.js 16 / React 19 frontend (Tailwind CSS v4, shadcn/ui) |
| `apps/api` | Rust API service (axum + tokio) |
| `packages/lint` | Shared Ultracite / Oxlint / Oxfmt config (`@pumapanel/lint`) |
| `compose.yaml` | Container stack for `web` + `api` (optional Traefik/nginx profiles) |
| `deploy/traefik` | Traefik static + dynamic config (ACME, routing, `/api` prefix strip) |
| `deploy/nginx` | nginx proxy config + TLS placeholders |
| `docs/` | Architecture, development, deployment, and reference documentation |
| `.github/` | CI/CD workflows, issue & PR templates, Dependabot config |

## Getting started

Toolchain versions (Node 24, pnpm 12, Rust 1.99) are pinned in `.moon/toolchains.yml` and installed through [proto](https://moonrepo.dev/proto).

```bash
moon setup        # install toolchains
pnpm install      # install JS/TS dependencies
moon run dev      # start all dev servers (web :3000, api :3001)
```

Open http://127.0.0.1:3000 for the web app — it proxies `/api/*` to the API, so registration, login, and the session cookie all work same-origin. The API is also reachable directly at http://127.0.0.1:3001 (`/health` and `/auth/*`). See `docs/architecture/web.md`.

## Common tasks

```bash
moon run web:build      # build the frontend
moon run api:test       # test the backend
moon run web:test       # test the frontend
moon run :lint          # lint every project
moon run :typecheck     # type-check every project
pnpm check              # lint + format check (Ultracite)
pnpm fix                # auto-fix lint + format issues
```

## Deployment

Build and run the whole stack with Docker or Podman:

```bash
cp .env.example .env                            # set JWT_SECRET (openssl rand -base64 48)
docker compose up -d --build                    # web + api only
docker compose --profile traefik up -d          # + Traefik on :80/:443
docker compose --profile nginx up -d            # + nginx on :80
```

Read [DEPLOYMENT.md](./DEPLOYMENT.md) for the full guide, and `docs/deployment/` for architecture, TLS, and operations details.

## Documentation

The `docs/` folder is the home for project documentation:

| Folder | Contents |
| --- | --- |
| [`docs/architecture/`](./docs/architecture/overview.md) | System overview, monorepo, web, API |
| [`docs/development/`](./docs/development/setup.md) | Setup, workflow, testing, code style |
| [`docs/deployment/`](./docs/deployment/README.md) | Deployment decisions, TLS, operations |
| [`docs/reference/`](./docs/reference/commands.md) | Commands, configuration, ports |

## Contributing

Contributions are welcome! Read [CONTRIBUTING.md](./CONTRIBUTING.md) for the contribution workflow, then check the [Code of Conduct](./CODE_OF_CONDUCT.md). Please use the issue templates for bugs and feature requests.

## Security

See [SECURITY.md](./SECURITY.md). Report vulnerabilities privately via [security advisories](https://github.com/ArcFOSS/pumapanel/security/advisories/new).

## License

Distributed under the [Apache License 2.0](./LICENSE).

![Contributor Covenant](https://shieldcn.dev/badge/Contributor_Covenant-2.1-4baaaa.svg) This project is governed by the Contributor Covenant.
