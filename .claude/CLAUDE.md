# PumaPanel

PumaPanel is a monorepo with a Next.js web frontend and a Rust API backend, orchestrated by [moon](https://moonrepo.dev) and [pnpm](https://pnpm.io) workspaces.

## Repository layout

| Path | Description |
| --- | --- |
| `apps/web` | Next.js 16 / React 19 frontend (Tailwind CSS v4, shadcn/ui) |
| `apps/api` | Rust API service (axum + tokio) |
| `packages/lint` | Shared Ultracite / Oxlint / Oxfmt config published as `@pumapanel/lint` |

## Toolchain

Versions are pinned in `.moon/toolchains.yml` and installed through [proto](https://moonrepo.dev/proto): Node 24, pnpm 12, Rust 1.99. Run `moon setup` once after cloning.

## Commands

```bash
moon setup                 # install toolchains
moon run dev               # start every dev server (web :3000, api :3001, agent :9091)
moon run web:build         # run a task for one project
moon run :lint             # lint every project
moon run :typecheck        # type-check every project
pnpm check                 # lint + format check the JS/TS workspace
pnpm fix                   # auto-fix lint + format issues
```

## Conventions

- Keep configuration for each project in its own `moon.yml`.
- Shared JS/TS lint and format config lives in `packages/lint`.
- Git hooks run Ultracite on staged files before every commit.

---

@../AGENTS.md
