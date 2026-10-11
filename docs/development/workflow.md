# Development Workflow

This page describes how changes move from an idea to `main`. A practical checklist lives in [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Branching

- `main` is the integration branch and is **protected**: no direct pushes, no force pushes, and pull requests need **one approving review**. Maintainers bypass the rules.
- Feature work goes on short-lived branches:
  - `fix/<short-name>` — bug fixes
  - `feat/<short-name>` — new features
  - `docs/<short-name>` — documentation
  - `chore/<short-name>` — tooling, housekeeping, dependencies

## Commit conventions

We use [Conventional Commits](https://www.conventionalcommits.org):

```text
type(scope): description

type:     feat | fix | refactor | docs | test | chore | ci | perf
scope:    web | api | lint | deploy | ci | repo (optional)
```

Examples:

```text
fix(api): keep in-flight requests alive during shutdown
feat(web): add theme toggle to the header
docs: add security policy
ci: cache proto toolchains between runs
```

Commits are checked by a **pre-commit hook** (Husky) that runs Ultracite on staged files; the formatter fixes and re-stages them automatically. Any issue that cannot be auto-fixed aborts the commit.

## The loop

1. **Branch** off latest `main`.
2. **Implement** with focused diffs — one logical change per PR.
3. **Format** early and often: `pnpm fix`.
4. **Verify** locally (the same gates CI runs):

   ```bash
   moon run :lint
   moon run :typecheck
   moon run api:test
   moon run web:build      # if apps/web changed
   docker compose config --quiet
   docker compose --profile traefik config --quiet
   docker compose --profile nginx config --quiet
   ```

5. **Update docs** where behaviour or usage changes (this folder).
6. **Open a pull request** using the [PR template](../../.github/pull_request_template.md).
7. **Iterate on review**, addressing feedback; updates to your branch re-trigger CI automatically.

## CI

`.github/workflows/ci.yml` runs on every push to `main` and on every PR:

| Job | What it runs |
| --- | --- |
| `verify` | `moon run :lint`, `moon run :typecheck`, `moon run api:test`, `moon run web:build` using the pinned proto toolchains |
| `containers` | `docker compose config` for the base + both proxy profiles, then builds the `api` and `web` images (GitHub Actions cache) |

`release.yml` publishes images to `ghcr.io/arcfoss/pumapanel-api` / `ghcr.io/arcfoss/pumapanel-web` and creates a release when a `v*` tag is pushed. See [Commands → Release](../reference/commands.md).

## Merging

- `main` requires a passing PR with at least one approving review.
- Status checks must be green before merge.
- Merged branches are deleted automatically; out-of-date PRs can be updated in-place (`allow_update_branch` is enabled).
