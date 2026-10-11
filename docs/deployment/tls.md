# TLS

The stack supports TLS in two different places, and you pick per deployment.

## Option A — Traefik terminates TLS (recommended)

When you enable the Traefik profile, Traefik listens on `:80` and `:443`, redirects HTTP → HTTPS, and obtains certificates from **Let's Encrypt** using the TLS-ALPN challenge.

1. Point a DNS record (e.g. an `A` record) at the host running the stack.
2. Set `DOMAIN=<your-domain>` in `.env` (defaults to `localhost`).
3. Ensure ports 80 and 443 are reachable from the internet.
4. Optionally customize the ACME resolver in `deploy/traefik/traefik.yml`:

```yaml
certificatesResolvers:
  letsencrypt:
    acme:
      email: you@example.com # for expiry notices (optional)
      storage: /letsencrypt/acme.json
      # caServer: https://acme-staging-v02.api.letsencrypt.org/directory  # uncomment to test
      tlsChallenge: {}
```

> The ACME settings **must** live in `deploy/traefik/traefik.yml` — Traefik ignores resolver config passed through CLI/env when the static config is loaded via `--configFile`.

Certificates and the ACME account persist in the `traefik-letsencrypt` volume.

## Option B — nginx with TLS upstream (default)

The bundled nginx config is **HTTP-only** by default: it proxies to `web` and `api`, and TLS is terminated **upstream** — by Traefik, a cloud load balancer, or another proxy in front of it. This keeps the nginx profile self-contained and free of certificate handling.

## Option C — nginx terminates TLS directly

When the nginx container must do the TLS termination itself:

1. Place `fullchain.pem` and `privkey.pem` in `deploy/nginx/certs/` (git-ignored) and generate or renew them however you like (certbot, Let's Encrypt DNS challenge, internal CA, …) — a cron/systemd unit on the host works well.
2. Uncomment the **HTTPS server block** in `deploy/nginx/conf.d/pumapanel.conf` and restart the service.
3. Publish the HTTPS port: add `"${HTTPS_PORT:-443}:443"` to the `nginx` `ports:` list in `compose.yaml`.

## Local development

- Traefik with `DOMAIN=localhost` will **fail the ACME TLS-ALPN challenge** (`example.com` and other reserved names are rejected by Let's Encrypt — this is expected). Use the **staging CA** in `traefik.yml`, or run without the Traefik profile and access `web` on loopback directly.
- Rootless Podman setups cannot bind `< 1024`: run with `HTTP_PORT=8080 HTTPS_PORT=8443` (see [DEPLOYMENT.md](../../DEPLOYMENT.md)).

## Security notes

- Add an `HSTS` header only when TLS is configured at the edge (the nginx config includes a commented example).
- Never commit private keys or Let's Encrypt account data; the cert volume and `deploy/nginx/certs/` are git-ignored.
