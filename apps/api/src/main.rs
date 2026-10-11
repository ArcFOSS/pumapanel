use std::env;

use axum::{routing::get, Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::signal;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "3001";

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Resolves when the process receives `SIGINT` (Ctrl+C) or `SIGTERM`.
///
/// Containers stop a service by sending `SIGTERM`, so handling it lets
/// in-flight requests finish before the process exits.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to `127.0.0.1` during local development, but allow containers to
    // listen on all interfaces via `HOST=0.0.0.0`.
    let host = env::var("HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = env::var("PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());
    let addr = format!("{host}:{port}");

    let app = Router::new().route("/health", get(health));
    let listener = TcpListener::bind(&addr).await?;

    println!("API listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
