use std::sync::Arc;

use agent::config::Config;
use agent::engine::CliEngine;
use agent::operations::Operations;
use agent::{detect, routes};
use tokio::net::TcpListener;
use tokio::signal;

/// Resolves when the process receives `SIGINT` (Ctrl+C) or `SIGTERM`.
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
    // Support a local `.env` for development; real deployments pass env vars.
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let detection = detect::detect(config.runtime_override);

    println!(
        "pumapanel-agent: runtime {}, host {}",
        detection
            .runtime()
            .map_or("none", agent::wire::Runtime::label),
        match detection.host_environment {
            agent::wire::HostEnvironment::BareMetal => "bare metal",
            agent::wire::HostEnvironment::DockerContainer => "Docker container",
            agent::wire::HostEnvironment::PodmanContainer => "Podman container",
        }
    );
    if let Some(message) = detection.unavailable_message() {
        eprintln!("warning: {message}");
    }

    let operations = Arc::new(Operations::new(
        Arc::new(CliEngine),
        detection,
        config.clone(),
    ));
    let token: Arc<str> = Arc::from(config.token.as_str());

    let listener = TcpListener::bind(&config.bind).await?;
    println!("pumapanel-agent: listening on {}", config.bind);

    axum::serve(listener, routes::router(operations, token))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
