# syntax=docker/dockerfile:1
#
# Build from the repository root so the workspace lockfile is available:
#   docker build -f apps/api/Dockerfile -t pumapanel/api .
#
# The final image only contains the compiled binary, CA certificates, and a
# curl binary used for the health check. It runs as an unprivileged user and
# stops on SIGTERM (see `with_graceful_shutdown` in src/main.rs).

# --- Build -------------------------------------------------------------------
FROM rust:1.99-slim AS builder

WORKDIR /app

# Compile dependencies first so they are cached independently of source edits.
COPY apps/api/Cargo.toml apps/api/Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY apps/api/src ./src
# Migrations are embedded at compile time by `sqlx::migrate!`, so they must be
# present when the real build runs (the dependency-cache build above uses a
# stub `main.rs` that does not reference them).
COPY apps/api/migrations ./migrations
RUN touch src/main.rs && cargo build --release --locked

# --- Runtime -----------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 10001 app \
    && useradd --uid 10001 --gid app --home-dir /app --shell /usr/sbin/nologin app

# Writable directory for the SQLite database. Compose mounts a named volume
# here; Docker/Podman initialise an empty volume with this directory's
# ownership, so the unprivileged `app` user can write to it.
RUN install -d -o app -g app /app/data

COPY --from=builder /app/target/release/api /usr/local/bin/api

WORKDIR /app
USER app

# `HOST=0.0.0.0` makes the service reachable from the container network.
ENV HOST=0.0.0.0 \
    PORT=3001 \
    DATABASE_URL=sqlite:///app/data/pumapanel.db

EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl --fail --silent http://127.0.0.1:3001/health || exit 1

ENTRYPOINT ["/usr/local/bin/api"]
