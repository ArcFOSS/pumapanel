# Contributing to PumaPanel

Thanks for your interest in contributing! This guide covers how to set up the project, what we expect from contributions, and how to get a change merged. Please also read the [Code of Conduct](./CODE_OF_CONDUCT.md) and the [documentation](./docs/README.md).

## Project orientation

PumaPanel is a monorepo with a Next.js web frontend and a Rust API backend, orchestrated by [moon](https://moonrepo.dev) and [pnpm](https://pnpm.io) workspaces.

| Path | Description |
| --- | --- |
| `apps/web` | Next.js 16 / React 19 frontend (Tailwind CSS v4, shadcn/ui) |
| `apps/api` | Rust API service (axum + tokio) |
| `packages/lint` | Shared Ultracite / Oxlint / Oxfmt config (`@pumapanel/lint`) |
| `deploy/` | Container deployment (Docker, Podman, Traefik, nginx) |

Start with `docs/architecture/overview.md` to understand the system, then look at the project-specific page for whatever you are changing (`docs/architecture/web.md` or `docs/architecture/api.md`).

## Local development setup

Toolchain versions (Node 24, pnpm 12, Rust 1.99) are pinned in `.moon/toolchains.yml` and installed through [proto](https://moonrepo.dev/proto).

```bash
moon setup        # install toolchains (run once after cloning)
pnpm install      # install JS/TS dependencies
moon run dev      # start all dev servers (web :3000, api :3001)
```

See `docs/development/setup.md` for troubleshooting and details.

## Common commands

```bash
moon run web:build      # build the frontend
moon run api:test       # test the backend (cargo test)
moon run :lint          # lint every project
moon run :typecheck     # type-check every project
pnpm check              # lint + format check (Ultracite)
pnpm fix                # auto-fix lint + format issues
```

## Making changes

1. **Create a branch** off `main`: `git checkout -b fix/your-fix` or `feat/your-feature`.
2. **Keep diffs focused.** One logical change per pull request — it is easier to review and easier to revert.
3. **Write a descriptive commit message.** We use [Conventional Commits](https://www.conventionalcommits.org) style: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`, `ci:`. Examples: `fix(api): handle empty health payload`, `docs: add deployment guide`.
4. **Run the checks before pushing:**

   ```bash
   pnpm fix                # auto-format staged changes
   moon run :lint          # lint + clippy everywhere
   moon run :typecheck
   moon run api:test
   moon run web:build      # when the web app changed
   ```

   A pre-commit hook runs Ultracite on staged files automatically. The CI workflow runs the same checks on every pull request.

5. **Update documentation** when behaviour or usage changes. Most pages live in `docs/`; deployment-specific docs live in `docs/deployment/` and `DEPLOYMENT.md`.

## Pull requests

- Fill out the [pull request template](./.github/pull_request_template.md). It reminds reviewers of the required checks.
- Link related issues with `Closes #123` / `Fixes #123` where applicable.
- Wait for CI to pass: **lint → typecheck → test → build → container validation**.
- `main` is protected: your PR needs **one approving review**. Maintainers can push fixes to your branch (`allow_update_branch` is enabled).

## Code style

- JavaScript/TypeScript: likely formatted and linted for you by Ultracite (Oxlint + Oxfmt). See `docs/development/code-style.md`.
- Rust: `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings`.
- Respect the project's Ultracite standards (accessible, performant, type-safe, maintainable code); a summary is in `AGENTS.md`.

## Reporting issues

- Use the issue templates: `Bug report` and `Feature request`. Blank issues are disabled.
- **Security vulnerabilities**: use the [private security advisory channel](https://github.com/ArcFOSS/pumapanel/security/advisories/new), not a public issue. See `SECURITY.md`.

## Code of conduct

All participants must follow our [Code of Conduct](./CODE_OF_CONDUCT.md). Be kind, be constructive, and focus on the code.
