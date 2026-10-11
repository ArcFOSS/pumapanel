# PumaPanel Documentation

Documentation for the PumaPanel monorepo: a Next.js web frontend and a Rust API backend orchestrated with moon and pnpm, deployable with Docker/Podman behind Traefik or nginx.

## Documentation map

| Section | Contents |
| --- | --- |
| [Architecture](./architecture/overview.md) | How the pieces fit together: system overview, the monorepo/toolchain, the web app, and the API |
| [Development](./development/setup.md) | Local setup, contribution workflow, testing, and code style |
| [Deployment](./deployment/README.md) | Container topology, proxy decisions, TLS, and operations |
| [Reference](./reference/commands.md) | Cheat sheets: commands, configuration, and ports |
| [Troubleshooting](./troubleshooting.md) | Common issues and their fixes |

## Getting around

- **New to the project?** Start with [Architecture overview](./architecture/overview.md), then [Development setup](./development/setup.md).
- **Deploying it?** Read the root [DEPLOYMENT.md](../DEPLOYMENT.md) for the practical guide, and the [deployment docs](./deployment/README.md) for the reasoning behind the setup.
- **Contributing?** See [CONTRIBUTING.md](../CONTRIBUTING.md) and [Development workflow](./development/workflow.md).
- **Looking for a command?** Go straight to [Reference: commands](./reference/commands.md).

## Conventions used in these docs

- Repo-root relative paths are used throughout (`docs/…`, `apps/web/…`).
- Commands assume a terminal in the repository root unless stated otherwise.
- `moon run <project>:<task>` addresses a single project's task; `moon run :<task>` runs it for every project that defines it.
