use std::sync::Arc;

use api::config::Config;
use api::db;
use api::state::AppState;
use tokio::net::TcpListener;
use tokio::signal;

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
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load a `.env` file for local development. This is a no-op when the file
    // is absent (for example inside a container, where configuration is
    // injected through the environment), and never overrides variables that are
    // already set.
    let _ = dotenvy::dotenv();

    let config = Config::from_env()?;
    let pool = db::init_pool(&config.database_url).await?;

    // Bind to `127.0.0.1` during local development, but allow containers to
    // listen on all interfaces via `HOST=0.0.0.0`.
    let address = format!("{}:{}", config.host, config.port);

    let state = AppState {
        db: pool,
        config: Arc::new(config),
    };

    let app = api::app(state);
    let listener = TcpListener::bind(&address).await?;

    println!("API listening on http://{address}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
