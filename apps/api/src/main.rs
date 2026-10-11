use axum::{routing::get, Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/health", get(health));

    let listener = TcpListener::bind("127.0.0.1:3001").await?;

    println!("API listening on http://127.0.0.1:3001");

    axum::serve(listener, app).await?;

    Ok(())
}

