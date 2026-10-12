# Deployment Operations

Day-to-day management of a running PumaPanel stack. Commands assume `docker compose`; swap `docker` → `podman` for Podman deployments.

## Build and start

```bash
cp .env.example .env                   # first time only

# web + api
docker compose up -d --build

# with a reverse proxy (enable exactly one)
docker compose --profile traefik up -d --build
docker compose --profile nginx   up -d --build
```

## Inspect

```bash
docker compose ps                      # status + health
docker compose logs -f web             # follow one service
docker compose logs -f                 # follow everything
curl -fsS http://127.0.0.1:3000/ | head   # web responds
curl -fsS http://127.0.0.1:3001/health     # → {"status":"ok"}
```

## Update a running stack

```bash
docker compose build --pull            # rebuild images on fresh base images
docker compose up -d                   # recreate changed containers only
docker compose ps                      # confirm web+api are (healthy)
```

## Backups

Persistent data lives in two named volumes:

- `pumapanel_api-data` — the SQLite database holding user accounts.
- `pumapanel_traefik-letsencrypt` — the ACME account and issued certificates.

```bash
# Accounts
docker run --rm -v pumapanel_api-data:/data -v "$PWD":/backup \
  alpine tar czf /backup/api-data-$(date +%F).tar.gz -C /data .

# Certificates
docker run --rm -v pumapanel_traefik-letsencrypt:/data -v "$PWD":/backup \
  alpine tar czf /backup/letsencrypt-$(date +%F).tar.gz -C /data .
```

(Check the exact volume names with `docker volume ls` — Compose prefixes them with the project name.) Restore by replacing the contents of the relevant volume and recreating the service.

## Recover / roll back

- Docker images are tagged `pumapanel/*:latest`. To pin a known-good build, override the image tag in `compose.yaml` (e.g. a SHA from your registry) and recreate; CI/CD can publish such tags — see [workflow → release](../development/workflow.md).
- To leave no trace: `docker compose down -v` removes containers _and_ the certificate volume.

## Validate configuration

```bash
docker compose config --quiet                       # base
docker compose --profile traefik config --quiet     # traefik
docker compose --profile nginx   config --quiet     # nginx

# nginx syntax (stub upstream hosts since they resolve at start-up)
docker run --rm --add-host web:127.0.0.1 --add-host api:127.0.0.1 \
  -v "$PWD/deploy/nginx/conf.d:/etc/nginx/conf.d:ro" nginx:1.29-alpine nginx -t
```

## Health checks and lifecycle

- `web` and `api` declare health checks both in their Dockerfiles and in `compose.yaml` (Podman's OCI images drop the image-level `HEALTHCHECK`).
- `docker compose stop`/`down` sends **SIGTERM**; the API finishes in-flight requests before exiting.

## Security checklist

1. `.env` **never** committed — `.env.example` ships safe defaults only.
2. Run the stack as an unprivileged user (images already do); don't add host sockets to containers.
3. Keep secrets out of `compose.yaml` (use `env_file`/`.env`).
4. Follow [SECURITY.md](../../SECURITY.md) for vulnerability reporting, and keep Dependabot-managed dependencies current.
