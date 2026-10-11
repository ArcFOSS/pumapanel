# PumaPanel

A monorepo with a Next.js web frontend and a Rust API backend, orchestrated by [moon](https://moonrepo.dev) and [pnpm](https://pnpm.io).

## Layout

| Path | Description |
| --- | --- |
| `apps/web` | Next.js 16 / React 19 frontend (Tailwind CSS v4, shadcn/ui) |
| `apps/api` | Rust API service (axum + tokio) |
| `packages/lint` | Shared Ultracite / Oxlint / Oxfmt config (`@pumapanel/lint`) |

## Getting started

Toolchain versions (Node 24, pnpm 12, Rust 1.99) are pinned in `.moon/toolchains.yml` and installed through [proto](https://moonrepo.dev/proto).

```bash
moon setup        # install toolchains
pnpm install      # install JS/TS dependencies
moon run :dev     # start all dev servers (web :3000, api :3001)
```

## Common tasks

```bash
moon run web:build      # build the frontend
moon run api:test       # test the backend
moon run :lint          # lint every project
moon run :typecheck     # type-check every project
pnpm check              # lint + format check (Ultracite)
pnpm fix                # auto-fix lint + format issues
```
