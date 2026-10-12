# Command Reference

All commands are run from the repository root. Toolchain commands resolve through proto/moon.

## Toolchains

| Command | Purpose |
| --- | --- |
| `moon setup` | Install the pinned toolchains (Node, pnpm, Rust) via proto |
| `moon --version` / `pnpm --version` / `cargo --version` | Sanity-check installed tools |

## Development

| Command                 | Purpose                                           |
| ----------------------- | ------------------------------------------------- |
| `moon run dev`          | Start both dev servers (web `:3000`, api `:3001`) |
| `moon run :dev`         | Same, via the all-projects scope                  |
| `moon run web:dev`      | Next.js dev server only                           |
| `moon run api:dev`      | `cargo run` only                                  |
| `pnpm --filter web dev` | Same as `web:dev` without moon                    |
| `pnpm install`          | Install JS/TS workspace dependencies              |

## Builds

| Command              | Purpose                            |
| -------------------- | ---------------------------------- |
| `moon run web:build` | `next build` (production)          |
| `moon run api:build` | `cargo build --release`            |
| `moon run :build`    | Everything above for every project |

## Quality gates

| Command | Purpose |
| --- | --- |
| `moon run :lint` | Oxlint (web) + `cargo clippy -D warnings` (api) |
| `moon run :typecheck` | `next typegen && tsc --noEmit` (web) + `cargo check` (api) |
| `moon run api:test` | `cargo test` |
| `moon run web:test` | `vitest run` |
| `pnpm check` | `ultracite check` over the JS/TS workspace |
| `pnpm fix` | `ultracite fix` (auto-format + auto-fix lint) |
| `pnpm exec ultracite doctor` | Diagnose Ultracite/Oxlint/Oxfmt setup |

## Containers (Docker / Podman)

Swap `docker` for `podman` where applicable.

| Command | Purpose |
| --- | --- |
| `docker compose up -d --build` | Build + start `web` + `api` |
| `docker compose --profile traefik up -d` | Add Traefik (`:80/:443`) |
| `docker compose --profile nginx up -d` | Add nginx (`:80`) |
| `docker compose ps` | Status + health |
| `docker compose logs -f [service]` | Follow logs |
| `docker compose config --quiet` | Validate Compose file (base) |
| `docker compose --profile traefik config --quiet` | Validate Traefik profile |
| `docker compose --profile nginx config --quiet` | Validate nginx profile |
| `docker compose build --pull` | Rebuild on fresh base images |
| `docker compose down [-v]` | Stop (–v also deletes the cert volume) |
| `podman build -f apps/api/Dockerfile .` | Build the API image with Podman |
| `BUILDAH_FORMAT=docker podman build …` | Keep image-level `HEALTHCHECK` on Podman |

## Validation

| Command | Purpose |
| --- | --- |
| `curl -fsS http://127.0.0.1:3001/health` | API health (`{"status":"ok"}`) |
| `curl -fsSI http://127.0.0.1:3000/login` | Web HTTP status (`/` redirects here when signed out) |
| `curl -fsS http://127.0.0.1:3000/api/auth/me` | Session check through the web origin (`401` when signed out) |
| `nginx -t` (see [operations](../deployment/operations.md)) | nginx config syntax |

## CI/CD

| Trigger | Workflow | Effect |
| --- | --- | --- |
| Push / PR to `main` | `.github/workflows/ci.yml` | lint → typecheck → test → build + container validation |
| Tag `v*` | `.github/workflows/release.yml` | push `ghcr.io/arcfoss/pumapanel-{api,web}` + GitHub release |
| Schedule | `.github/dependabot.yml` | weekly/monthly dependency PRs |

```bash
git tag v0.1.0 && git push origin v0.1.0   # trigger a release
```
