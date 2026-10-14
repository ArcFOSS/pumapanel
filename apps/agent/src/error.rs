use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Errors returned by the agent's HTTP API.
///
/// Mirrors the main API's `{ "error": { "code", "message" } }` body so the
/// client can handle both services the same way. Internal detail is logged and
/// never sent to the caller.
#[derive(Debug)]
pub enum AgentError {
    /// 401 — missing or wrong bearer token.
    Unauthorized,
    /// 400 — the request was malformed or failed validation.
    BadRequest(String),
    /// 404 — no managed container for the requested server id.
    NotFound,
    /// 503 — no container runtime is available to run servers.
    Unavailable(String),
    /// 502 — the container runtime rejected a command.
    Runtime(String),
    /// 500 — unexpected failure.
    Internal(String),
}

impl AgentError {
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal(detail.into())
    }
}

impl IntoResponse for AgentError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "A valid agent token is required".to_string(),
            ),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "validation_error", message),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "No server with that id is managed by this agent".to_string(),
            ),
            Self::Unavailable(message) => {
                (StatusCode::SERVICE_UNAVAILABLE, "runtime_unavailable", message)
            }
            Self::Runtime(message) => (StatusCode::BAD_GATEWAY, "runtime_error", message),
            Self::Internal(detail) => {
                eprintln!("error: {detail}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        (
            status,
            Json(serde_json::json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}
