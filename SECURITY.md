# Security Policy

PumaPanel takes security seriously. This document explains which versions are supported and how to report a vulnerability.

## Supported versions

The project is pre-1.0 and does not maintain release branches. The **only** supported version is the latest state of `main`, plus the most recent version-tagged release for purposes of regression reporting. If you rely on a pinned release, plan to upgrade to the latest release when one is published.

| Version               | Supported |
| --------------------- | --------- |
| `main` (latest)       | ✅        |
| Latest tagged release | ✅        |
| Older releases        | ❌        |

## Reporting a vulnerability

**Do not open a public issue** for security problems. Instead, use GitHub's private reporting channel:

<https://github.com/ArcFOSS/pumapanel/security/advisories/new>

Please include as much of the following as possible:

- Affected component (`apps/web`, `apps/api`, deployment, tooling)
- Version or commit SHA affected, and whether the issue is present on `main`
- Steps to reproduce, including a minimal example if possible
- Impact (what an attacker can gain) and any proposed remediation

You will receive an acknowledgement within **7 days**. We work to triage, confirm, and release a fix as quickly as possible, and we coordinate disclosure with you before publishing details.

### What is in scope

- Code in `apps/`, `packages/`, `deploy/`, `.github/workflows/`, and the root configuration files (`compose.yaml`, Dockerfiles, `.env.example`)
- Running services: the Next.js web app and the Rust API, and their container images

### What is out of scope

- Third-party dependencies — report those to their respective projects (or rely on Dependabot + GitHub security updates, which are enabled)
- Your own deployment/hosting environment and its credentials

## Security practices in this repository

- **No secrets in git.** Configuration lives in `.env` (git-ignored) and `deploy/`. Committed files contain no passwords, tokens, or private keys. `.env.example` ships safe example values only.
- **Secret scanning** is enabled on the repository, with push protection.
- **Dependabot** security updates and version updates are enabled.
- **Unprivileged containers.** Both images run as a non-root user, and the API binds 127.0.0.1 by default (0.0.0.0 only inside the container network).
- **Dependency hygiene.** Toolchain versions are pinned in `.moon/toolchains.yml`; lockfiles are committed for npm (`pnpm-lock.yaml`) and cargo (`apps/api/Cargo.lock`).

When reporting, please bear in mind that this is a small open-source project run by volunteers — we appreciate patience and detailed, constructive reports.
