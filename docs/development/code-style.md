# Code Style

PumaPanel uses **Ultracite** — a zero-config linting/formatting preset for JavaScript/TypeScript built on [Oxlint](https://oxc.rs) + [Oxfmt](https://oxc.rs). Rust code is governed by `rustfmt` + `clippy`.

## Quick reference

```bash
pnpm exec ultracite check      # see what's wrong
pnpm exec ultracite fix        # auto-fix formatting + fixable lint issues
pnpm check                     # == `ultracite check` (workspace-wide)
pnpm fix                       # == `ultracite fix` (workspace-wide)
```

On commit, a Husky pre-commit hook formats staged files automatically.

## How the config is layered

```text
ultracite preset
   └── packages/lint/oxlint.config.mjs   (shared, published as @pumapanel/lint)
          ├── root oxlint.config.mts     (re-exports @pumapanel/lint)
          └── apps/web/oxlint.config.mts (re-exports @pumapanel/lint)
```

Notable overrides in `packages/lint`:

- **shadcn/ui components** (`apps/web/src/components/ui/**`) use baseline defaults instead of strict rules (e.g. SonarJS) to keep generated UI code stable.
- React-diagnostic and accessibility rules from the Ultracite preset remain active for application code.

## Principles

Ultracite enforces most of this mechanically; keep the rest in mind:

1. **Type-safe and explicit** — use real types, prefer `unknown` over `any`, narrow with control flow, avoid type assertions, extract named constants instead of magic numbers.
2. **Modern JS/TS** — arrow functions for callbacks, `for...of` over `.forEach`, optional chaining, template literals, destructuring, `const` by default.
3. **Async correctness** — always `await` promises, prefer `async`/`await`, handle errors with meaningful `try/catch`.
4. **Accessible React** — semantic HTML + ARIA, real `<button>`/`<nav>`, keyboard handlers alongside mouse handlers, meaningful alt text, correct heading hierarchy.
5. **Clean code** — early returns, small focused functions, extracted boolean conditions, no `console.log`/`debugger` in committed code, no `dangerouslySetInnerHTML` unless necessary.
6. **Performance** — no spread-accumulators in loops, top-level regex literals, specific imports (no barrel files), Next `<Image>` over `<img>`.

## Rust

- **Format:** `cargo fmt`.
- **Lint:** `cargo clippy --all-targets --all-features -- -D warnings` (`moon run api:lint`).
- Follow `clippy` without exceptions — warnings are treated as errors in CI.
- Use `std::env` + `const` for configuration defaults (see `apps/api/src/main.rs`), `Result<_, Box<dyn Error>>` in `main`, and `#[derive(Serialize)]` for JSON payloads.

## Toolchain pinning

Style is not the only thing pinned: Node 24.21.0, pnpm 12.12.0, and Rust 1.99.0 are locked in `.moon/toolchains.yml` so local, CI, and container builds all behave identically. See [Monorepo & toolchain](../architecture/monorepo.md).
