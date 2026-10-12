# Deployment

PumaPanel ships as two OCI images built from the repository root:

| Service | Image | Listens | Exposed by default |
| --- | --- | --- | --- |
| `web` | `pumapanel/web:latest` | `:3000` (Next.js standalone server) | loopback `127.0.0.1:3000` |
| `api` | `pumapanel/api:latest` | `:3001` (axum) | internal network only |

Both images run as an unprivileged user, expose Docker health checks, and shut down gracefully on `SIGTERM`. Everything is driven by `compose.yaml`, which works with Docker and Podman.

- `api` binds `0.0.0.0:3001` inside the container and serves `GET /health` plus the `/auth/*` email/password endpoints (registration, login, logout, session).
- `web` is the public entry point. `api` is intentionally **not** published to the host; reach it through the container network or a reverse proxy.

## Prerequisites

- Docker Engine with Compose v2, or Podman with a Compose provider.
- For TLS certificates via Let's Encrypt: a public DNS record pointing at the host, and inbound ports 80 and 443 reachable.

## Configuration

Copy the example environment file and edit it:

```bash
cp .env.example .env
```

| Variable | Default | Purpose |
| --- | --- | --- |
| `DOMAIN` | `example.com` | Public hostname used by Traefik for routing and certificates. |
| `HTTP_PORT` | `80` | Host port for the proxy's HTTP entrypoint. |
| `HTTPS_PORT` | `443` | Host port for Traefik's HTTPS entrypoint. |
| `WEB_PORT` | `3000` | Loopback port for direct access to the web app when no proxy runs. |
| `JWT_SECRET` | — | **Required.** Signing key for session JWTs (≥32 chars). Generate with `openssl rand -base64 48`. |
| `JWT_EXPIRATION_SECS` | `86400` | Session token/cookie lifetime in seconds. |
| `COOKIE_SECURE` | `false` | Set to `true` when served over HTTPS (Traefik, or TLS upstream of nginx) so the session cookie gets the `Secure` attribute. |

`.env` is git-ignored. The API refuses to start when `APP_ENV=production` (the Compose default) and `JWT_SECRET` is missing or too short, so `docker compose up` will not come up until you set it. The ACME (Let's Encrypt) resolver — including the optional contact email and the staging endpoint switch — is configured in `deploy/traefik/traefik.yml`, because Traefik cannot read that value from the environment.

## Docker

```bash
# Build and start web + api
docker compose up -d --build

# Follow logs
docker compose logs -f

# Confirm health
docker compose ps

# Stop and remove (add -v to also delete the certificate volume)
docker compose down
```

The web app is then available at <http://127.0.0.1:3000> (change `WEB_PORT` to taste).

## Podman

The same `compose.yaml` and images are used; there is no separate Podman file. `podman compose` delegates to a Compose provider and talks to the Podman service. On a rootless setup:

```bash
# Make sure the Podman socket is running
systemctl --user enable --now podman.socket

# Rootless Podman cannot bind ports below 1024, so use unprivileged ports
HTTP_PORT=8080 HTTPS_PORT=8443 WEB_PORT=3000 podman compose up -d --build
```

Notes specific to Podman:

- **Rootless ports.** Ports `< 1024` require either unprivileged ports as above, or lowering `net.ipv4.ip_unprivileged_port_start` on the host.
- **SELinux.** Bind mounts in `compose.yaml` use the `:Z` suffix so files are relabeled on SELinux hosts. The option is ignored where SELinux is disabled or on Docker Desktop.
- **Image format / health checks.** `podman build` produces OCI images by default and discards the `HEALTHCHECK` baked into the Dockerfile, so the health checks are also declared in `compose.yaml` and `depends_on: condition: service_healthy` keeps working. To keep an image-level health check (for `podman run`), build with `BUILDAH_FORMAT=docker podman build ...`.
- **Socket.** Traefik uses the _file_ provider and does not need access to any container-management socket, so no `podman.sock` mount is required.

## Reverse proxy

`compose.yaml` defines two optional, mutually exclusive profiles. Enable **one** of them: both want the host's `HTTP_PORT`/`HTTPS_PORT`.

### Traefik

```bash
docker compose --profile traefik up -d
```

Traefik is bundled in the stack, terminates TLS on `:443`, redirects `:80` to HTTPS, and obtains certificates from Let's Encrypt using the TLS-ALPN challenge. Routing is defined in `deploy/traefik/dynamic.yml`:

- `https://$DOMAIN/` → `web:3000`
- `https://$DOMAIN/api/` → `api:3001` (the `/api` prefix is stripped, so `/api/health` reaches the API as `/health`)

It reads the `DOMAIN` environment variable, so it works with Docker and Podman without editing files. Set a real `DOMAIN` in `.env` before enabling the profile, and adjust the ACME contact/staging settings in `deploy/traefik/traefik.yml`.

If you already run Traefik elsewhere, copy `deploy/traefik/dynamic.yml` and point the `loadBalancer` servers at whatever addresses resolve to `web:3000` and `api:3001`.

### nginx

```bash
docker compose --profile nginx up -d
```

nginx listens on `HTTP_PORT` (default `80`) and proxies to the same two services. **TLS is terminated upstream by default** (Traefik, a load balancer, or another proxy), so the bundled nginx config is HTTP-only. To terminate TLS directly in nginx:

1. Put `fullchain.pem` and `privkey.pem` in `deploy/nginx/certs/`.
2. Uncomment the HTTPS server block in `deploy/nginx/conf.d/pumapanel.conf`.
3. Add `"${HTTPS_PORT:-443}:443"` to the `nginx` service `ports` in `compose.yaml`.

## Ports

| Port | Service | Reachability |
| --- | --- | --- |
| `3000` | web | Published on loopback by default; keep internal when behind a proxy. |
| `3001` | api | Container network only. |
| `80` / `443` | Traefik (`--profile traefik`) | Public; TLS terminated here. |
| `80` | nginx (`--profile nginx`) | Public; TLS terminated upstream. |

## Persistent storage

User accounts are stored in SQLite on the named volume `api-data` (mounted at `/app/data`), so they survive image rebuilds. Migrations are embedded in the API binary and applied automatically on start-up.

The TLS stack keeps its own volume: `traefik-letsencrypt` stores the ACME account and issued certificates at `/letsencrypt/acme.json`. Back it up if you want to avoid re-issuing certificates.

`docker compose down` keeps both volumes; `docker compose down -v` deletes them (removing accounts and certificates).

## Health, logs, and lifecycle

- Health: each image defines a `HEALTHCHECK`. `docker compose ps` shows `(healthy)`; the proxy services wait for the backends to be healthy before starting (`depends_on: condition: service_healthy`).
- Logs: `docker compose logs -f [service]`.
- Stop safely: `docker compose stop` (or `down`) sends `SIGTERM`; the API finishes in-flight requests before exiting.

## Updating

```bash
docker compose build --pull     # or: podman compose build --pull
docker compose up -d
```

## Validating configuration

```bash
# Compose file(s)
docker compose config --quiet
docker compose --profile traefik config --quiet
docker compose --profile nginx config --quiet

# nginx syntax (upstream names are resolved at start-up, so provide stub hosts)
docker run --rm --add-host web:127.0.0.1 --add-host api:127.0.0.1 \
  -v "$PWD/deploy/nginx/conf.d:/etc/nginx/conf.d:ro" nginx:1.29-alpine nginx -t
```
