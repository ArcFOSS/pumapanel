# Deployment

PumaPanel ships as two OCI images and runs identically under **Docker** and **Podman** via a single `compose.yaml`. Two reverse proxies — **Traefik** and **nginx** — are provided as mutually exclusive Compose profiles.

> **Practical guide first:** [DEPLOYMENT.md](../../DEPLOYMENT.md) is the command-by-command reference (build, run, validate). The pages in this folder explain the _why_ behind the setup.

## Quick decision guide

| You want… | Use |
| --- | --- |
| Local containers, no domain | `docker compose up -d --build` (web on `127.0.0.1:3000`) |
| Automatic HTTPS with Let's Encrypt | Traefik profile (`DOMAIN` points at this host) |
| A lightweight HTTP proxy / existing TLS upstream | nginx profile |
| Rootless Podman | Set `HTTP_PORT`/`HTTPS_PORT` to `8080`/`8443` |

## Pages

| Page | Contents |
| --- | --- |
| [Architecture](./architecture.md) | How the images, network, health checks, and profiles fit together |
| [TLS](./tls.md) | Certificate strategy: Traefik ACME, upstream TLS, local development |
| [Operations](./operations.md) | Building, updating, logs, backups, rollback, and validation |

## Core facts

- `web` publishes `127.0.0.1:${WEB_PORT:-3000}:3000`; `api` is **not** published to the host at all.
- Both images run as an **unprivileged user** with a built-in `HEALTHCHECK`.
- Health checks are also declared in `compose.yaml`, because Podman's default OCI build format drops image-level `HEALTHCHECK`.
- Traefik uses the **file provider** — no container/`podman.sock` mount — so the same config works on Docker and Podman.
- Only **one** proxy profile can be active at a time (both bind `HTTP_PORT` / `HTTPS_PORT`).
