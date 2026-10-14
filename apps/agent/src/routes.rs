//! The agent's HTTP API.
//!
//! Everything except `/health` requires `Authorization: Bearer <AGENT_TOKEN>`.
//! The token is enforced by one middleware rather than per handler, so a new
//! route cannot accidentally ship unauthenticated.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::AgentError;
use crate::operations::Operations;
use crate::wire::{CreateServerRequest, RuntimeInfo, ServerList, ServerState};

/// Builds the agent's router.
pub fn router(operations: Arc<Operations>, token: Arc<str>) -> Router {
    let protected = Router::new()
        .route("/runtime", get(runtime))
        .route("/servers", get(list_servers).post(create_server))
        .route("/servers/{id}/start", post(start_server))
        .route("/servers/{id}/stop", post(stop_server))
        .route("/servers/{id}", delete(remove_server))
        .layer(middleware::from_fn_with_state(token, authorize))
        .with_state(operations);

    // Unauthenticated so container health checks and the API's start-up probe
    // can reach it without provisioning a token.
    Router::new()
        .route("/health", get(health))
        .merge(protected)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn runtime(State(operations): State<Arc<Operations>>) -> Json<RuntimeInfo> {
    Json(operations.info().await)
}

async fn list_servers(
    State(operations): State<Arc<Operations>>,
) -> Result<Json<ServerList>, AgentError> {
    Ok(Json(operations.list().await?))
}

async fn create_server(
    State(operations): State<Arc<Operations>>,
    Json(payload): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerState>), AgentError> {
    let state = operations.create(&payload).await?;
    Ok((StatusCode::CREATED, Json(state)))
}

async fn start_server(
    State(operations): State<Arc<Operations>>,
    Path(id): Path<String>,
) -> Result<Json<ServerState>, AgentError> {
    Ok(Json(operations.start(&id).await?))
}

async fn stop_server(
    State(operations): State<Arc<Operations>>,
    Path(id): Path<String>,
) -> Result<Json<ServerState>, AgentError> {
    Ok(Json(operations.stop(&id).await?))
}

/// Query for `DELETE /servers/{id}`; data removal is opt-in per request.
#[derive(Debug, Deserialize)]
struct RemoveQuery {
    #[serde(default, rename = "removeData")]
    remove_data: bool,
}

async fn remove_server(
    State(operations): State<Arc<Operations>>,
    Path(id): Path<String>,
    Query(query): Query<RemoveQuery>,
) -> Result<StatusCode, AgentError> {
    operations.remove(&id, query.remove_data).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Rejects requests that do not carry the shared token.
async fn authorize(
    State(expected): State<Arc<str>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AgentError> {
    let presented = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match presented {
        Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
            Ok(next.run(request).await)
        }
        _ => Err(AgentError::Unauthorized),
    }
}

/// Compares two byte strings without short-circuiting, so a wrong token cannot
/// be guessed one byte at a time from response timing.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_comparison_matches_only_exact_values() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secrez"));
        assert!(!constant_time_eq(b"secret", b"secret-longer"));
        assert!(!constant_time_eq(b"", b"x"));
    }
}
