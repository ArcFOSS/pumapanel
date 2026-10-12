pub mod admin;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod state;
pub mod users;

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Builds the application router with all routes and shared state attached.
///
/// Kept in the library (rather than `main.rs`) so integration tests can drive
/// the real router without binding a socket.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(auth::router())
        .merge(admin::router())
        .with_state(state)
}
