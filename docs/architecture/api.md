# API (`apps/api`)

The backend is a minimal Rust HTTP service built with [axum](https://github.com/tokio-rs/axum) 0.8 on [tokio](https://tokio.rs).

## What it does today

- Serves `GET /health` → `{"status":"ok"}`.
- Listens on `http://{HOST}:{PORT}`.
- Shuts down gracefully on `SIGINT` (Ctrl+C) and `SIGTERM` (containers).

The route table is deliberately small — axum makes it easy to grow without touching the container or proxy configuration.

## Behavior

`apps/api/src/main.rs`:

- **Binding.** `HOST` and `PORT` environment variables, defaulting to `127.0.0.1:3001`. Containers set `HOST=0.0.0.0` so the proxy network can reach the service; local development stays on loopback.
- **Graceful shutdown.** Both `CTRL_C` and `unix::SignalKind::terminate()` are awaited inside `tokio::select!`. When a container stops with `SIGTERM`, in-flight requests finish before the process exits (verified: exit code 0).
- **Responses.** `serde` + `serde_json` serialize the health payload.

## Dependencies (`Cargo.toml`)

| Crate | Version | Purpose |
| --- | --- | --- |
| `axum` | 0.8 | HTTP framework |
| `tokio` | 1.x | async runtime (`macros`, `rt-multi-thread`, `signal`) |
| `serde` / `serde_json` | 1.x | JSON serialization |
| `rayon` | 1.x | data parallelism (reserved for future workloads) |

Edition `2024` with the Rust 1.99 toolchain from `.moon/toolchains.yml`.

## Quality gates

- `moon run api:test` — `cargo test`.
- `moon run api:lint` — `cargo clippy --all-targets --all-features -- -D warnings`.
- `moon run api:typecheck` — `cargo check --all-targets`.
- `moon run api:build` — `cargo build --release`.

## See also

- [Deployment architecture](../deployment/architecture.md) — packaging and the `HOST`/`PORT` contract with Compose.
- [Reference: ports](../reference/ports.md) — where the API listens.
