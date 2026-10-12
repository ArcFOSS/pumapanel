# Development Setup

## Prerequisites

- A Linux or macOS machine (Windows works via WSL2).
- [moon](https://moonrepo.dev) and [proto](https://moonrepo.dev/proto) — or let moon install toolchains for you.
- Optional but recommended: [Docker](https://www.docker.com) or [Podman](https://podman.io) to run the container stack.

Toolchain versions are pinned in `.moon/toolchains.yml` (Node 24.21.0, pnpm 12.12.0, Rust 1.99.0) and installed through proto.

## First-time setup

```bash
git clone https://github.com/ArcFOSS/pumapanel.git
cd pumapanel

moon setup        # install the pinned toolchains via proto
pnpm install      # install JS/TS workspace dependencies
```

> If `moon`/`pnpm` are not on your `PATH` yet, follow the quick install instructions at <https://moonrepo.dev> and <https://pnpm.io>.

## Run the dev servers

```bash
moon run dev
```

This starts, in parallel:

| Service | URL                                                 |
| ------- | --------------------------------------------------- |
| `web`   | http://127.0.0.1:3000 (Next dev server)             |
| `api`   | http://127.0.0.1:3001 (axum, `/health` + `/auth/*`) |

Run one at a time with `moon run web:dev` or `moon run api:dev`.

`moon run dev` is shorthand for the root aggregator task, which depends on both `web:dev` and `api:dev`. `moon run :dev` (the all-projects scope) runs the same action graph, so either spelling starts each server exactly once.

In dev the web app rewrites `/api/*` to the API (default `http://127.0.0.1:3001`), so the browser uses the same origin and the session cookie stays same-origin: open http://127.0.0.1:3000/register to create an account, or hit the API directly at `http://127.0.0.1:3001/health`.

## Editor setup

- **VS Code** — `.vscode/settings.json` configures the workspace. Install the Ultracite extension (or `oxc` tools) for linting/formatting on save.
- **Zed** — `.zed/settings.json` is provided for the same purpose.
- **Cursor** — `tools` + hook wiring live in `.cursor/hooks.json` (and `.github/hooks/ultracite.json` for agent-driven formatting).

All three editors are pre-configured to format with Oxfmt and flag Oxlint issues using the shared `@pumapanel/lint` config.

## Verify your setup

```bash
moon run :lint            # lint + clippy everywhere
moon run :typecheck       # type-check everything
moon run api:test         # cargo test
moon run web:build        # Next.js production build
pnpm check                # Ultracite check over the workspace
```

## Troubleshooting

See [Troubleshooting](../troubleshooting.md) for common issues such as `ERR_PNPM_NO_AGENT_SKILLS_DIR` and missing toolchains.
