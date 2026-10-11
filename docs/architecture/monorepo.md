# Monorepo & Toolchain

PumaPanel is a monorepo managed by [moon](https://moonrepo.dev) and [pnpm](https://pnpm.io), with toolchains pinned and installed through [proto](https://moonrepo.dev/proto).

## Repository layout

```text
apps/
  web/          Next.js 16 frontend (TypeScript, Tailwind CSS v4, shadcn/ui)
  api/          Rust API (axum + tokio)
packages/
  lint/         Shared Ultracite/Oxlint/Oxfmt config → @pumapanel/lint
deploy/         Traefik + nginx configuration for the container stack
docs/           This documentation
.github/        CI/CD workflows, issue/PR templates, Dependabot config
```

The workspace is declared in `pnpm-workspace.yaml`:

```yaml
packages:
  - apps/*
  - packages/*
```

## moon projects

`.moon/workspace.yml` registers two projects:

| Project | Directory  | Language                 |
| ------- | ---------- | ------------------------ |
| `web`   | `apps/web` | TypeScript (application) |
| `api`   | `apps/api` | Rust (application)       |

Each project declares its tasks in its own `moon.yml`:

| Task        | `web`                          | `api`                         |
| ----------- | ------------------------------ | ----------------------------- |
| `dev`       | `pnpm dev` (Next dev, :3000)   | `cargo run` (:3001)           |
| `build`     | `next build`                   | `cargo build --release`       |
| `lint`      | `ultracite check .`            | `cargo clippy -- -D warnings` |
| `typecheck` | `next typegen && tsc --noEmit` | `cargo check --all-targets`   |
| `test`      | —                              | `cargo test`                  |
| `start`     | `next start`                   | —                             |

The `web:typecheck` task runs `next typegen` first, because `tsc` consumes route/`LayoutProps` types that Next only emits into `.next/types/`. This keeps type-checking working on a clean checkout and in CI without a prior build.

## Pinned toolchains

`.moon/toolchains.yml` pins every tool so local, CI, and container builds all use the same versions:

```yaml
node:
  version: "24.21.0"
pnpm:
  version: "12.12.0"
rust:
  version: "1.99.0"
```

- `moon setup` installs them through proto.
- GitHub Actions uses `moonrepo/setup-toolchain@v0` to install and cache them.
- The Docker images pin their own base images (`node:24.21.0-bookworm-slim`, `rust:1.99-slim`) so the runtime does not depend on the host toolchain. See [Deployment architecture](../deployment/architecture.md).

## Shared lint package

`packages/lint` publishes `@pumapanel/lint`, re-exported from the root `oxlint.config.mts` / `oxfmt.config.mts`. It layers the [Ultracite](https://github.com/Ultracite/ultracite) preset on top of Oxlint + Oxfmt, plus intentional overrides — for example, shadcn/ui components in `apps/web/src/components/ui/**` get baseline defaults instead of strict SonarJS rules. See [Code style](../development/code-style.md).

## pnpm workspace details

- `packageManager` is pinned to `pnpm@12.12.0`.
- Agent skill directories are declared explicitly (`.agents/skills`, `.claude/skills`) so `pnpm install` succeeds in CI and in container images without depending on auto-detection.
- `pnpm check` / `pnpm fix` run Ultracite at the workspace root and are the primary JS/TS quality gates.

## Lockfiles

- `pnpm-lock.yaml` — JS/TS dependencies (committed).
- `apps/api/Cargo.lock` — Rust dependencies (committed).

These lockfiles make installs and builds deterministic in CI and containers.
