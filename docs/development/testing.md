# Testing

PumaPanel keeps tests intentionally small today. Every layer is covered by a fast, scriptable gate that CI also runs.

## Rust tests (`apps/api`)

```bash
moon run api:test        # cargo test
```

There are no unit tests yet — the service currently exposes a single `/health` route. When adding endpoints, follow these conventions:

- Write unit tests next to the code with `#[cfg(test)]` modules.
- Prefer mocks via traits for anything external (HTTP clients, storage).
- Keep tests deterministic: no real network or clock reads.

Example skeleton:

```rust
#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_returns_ok() {
        // build an app fixture and call it with `oneshot(...)`
    }
}
```

(`axum` extracts/extensions come from `tower` — add it as a dev-dependency when you start writing request tests.)

## Frontend (`apps/web`)

No test runner is wired in yet — the web quality gates are:

```bash
moon run web:typecheck   # next typegen + tsc --noEmit
moon run web:lint        # Ultracite check
moon run web:build       # production build (catches import/rendering errors)
```

When you add a test runner, prefer one that fits the stack (e.g. [Vitest](https://vitest.dev) + [Testing Library](https://testing-library.com)) and register it as a moon `test` task.

## Container smoke tests

The fastest end-to-end validation of the whole stack:

```bash
docker compose up -d --build
docker compose ps                        # both services should show (healthy)
curl -fsS http://127.0.0.1:3000/ | grep -qi pumapanel && echo web-ok
curl -fsS http://127.0.0.1:3001/health   # → {"status":"ok"}
docker compose down
```

For proxy validation see [Deployment → operations](../deployment/operations.md).

## What CI runs

- lint (Oxlint + clippy with `-D warnings`)
- type-check (TS + Rust)
- API tests
- web production build
- Compose config validation (base + both proxy profiles)
- Container image builds

If it fails locally, it fails in CI — and vice versa.
