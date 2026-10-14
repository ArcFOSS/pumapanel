# Port Reference

Every port PumaPanel uses, in development and in containers.

| Port | Service | Local dev | Containers (`compose.yaml`) | Reachable from |
| --- | --- | --- | --- | --- |
| `3000` | `web` | Next dev server | Next standalone server | Host loopback (`127.0.0.1:${WEB_PORT:-3000}`) |
| `3001` | `api` | axum (default `127.0.0.1:3001`) | axum (`0.0.0.0:3001`) | Container network only |
| `9091` | `agent` | axum agent (default `127.0.0.1:9091`) | not in the Compose stack yet | Host loopback (`127.0.0.1:9091`) in dev |
| `80` | `traefik` (profile) | — | HTTP entrypoint → 301 to HTTPS | Host `${HTTP_PORT:-80}` |
| `443` | `traefik` (profile) | — | HTTPS entrypoint (TLS-ALPN) | Host `${HTTPS_PORT:-443}` |
| `80` | `nginx` (profile) | — | HTTP proxy to `web`/`api` | Host `${HTTP_PORT:-80}` |

## Notes

- **`api` is never published to the host** in the containerized stack; reach it through the container network (`http://api:3001`) or a proxy.
- **`web` publishes only on loopback**, so a browser on the host can reach it directly, but remote clients cannot — put a proxy or inbound rule in front for public access.
- **Rootless Podman** cannot bind `< 1024`: use `HTTP_PORT=8080` / `HTTPS_PORT=8443` (or lower `net.ipv4.ip_unprivileged_port_start`).
- App-internal port numbers are fixed by `HOST`/`PORT` (api) and `HOSTNAME`/`PORT` (web) environment variables — only the **host-side** ports are configurable through `.env`.

## Traffic flow

```text
internet ─▶ :80 ─▶ :443 ─▶ Traefik/nginx ─▶ web:3000  (browser, /)
                                      └──▶ api:3001   (backend, /api/*)
        OR  browser ─▶ 127.0.0.1:3000 ─▶ web (no proxy)
```
