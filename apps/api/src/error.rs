use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// A single field-level validation failure.
#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// Errors returned by the API.
///
/// Variants map to HTTP status codes and are rendered as a consistent JSON body:
/// `{ "error": { "code": "...", "message": "...", "fields": [...] } }`.
///
/// Internal details (database errors, hashing failures, ...) are never sent to
/// the client; only [`ApiError::Internal`] is rendered as a generic 500 after
/// the detail has been logged.
#[derive(Debug)]
pub enum ApiError {
    /// 400 — request failed validation. Body lists the offending fields.
    Validation(Vec<FieldError>),
    /// 400 — the container runtime rejected the command (a port already in
    /// use, a duplicate name, ...). Because the request caused it, the
    /// runtime's own message is safe to return and is the most useful one.
    Runtime(String),
    /// 401 — invalid login credentials. Deliberately generic.
    InvalidCredentials,
    /// 401 — missing, malformed, expired, or otherwise invalid token.
    Unauthorized,
    /// 403 — authenticated, but the account lacks the required role.
    Forbidden,
    /// 404 — the requested resource does not exist.
    NotFound,
    /// 409 — an account with this email already exists.
    EmailTaken,
    /// 409 — an account with this username already exists.
    UsernameTaken,
    /// 502 — the agent, or the PaperMC API, failed or answered unintelligibly.
    Upstream(String),
    /// 503 — no agent is configured, it cannot be reached, or it has no
    /// container runtime to run servers with.
    Unavailable(String),
    /// 500 — unexpected server-side failure.
    Internal(String),
}

impl ApiError {
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal(detail.into())
    }

    /// The 503 raised when a deployment has no agent configured.
    pub fn agent_unconfigured() -> Self {
        Self::Unavailable(
            "Server management is disabled: AGENT_URL is not configured for this panel.".to_string(),
        )
    }

    /// A short, human-readable reason for this error.
    ///
    /// Used where an error is *reported as a status* rather than returned as a
    /// failure — for example the runtime banner on the servers page, which
    /// explains why servers cannot be started instead of failing the request.
    pub fn reason(&self) -> String {
        match self {
            Self::Validation(fields) => fields
                .first()
                .map(|field| field.message.clone())
                .unwrap_or_else(|| "Validation failed".to_string()),
            Self::Runtime(message) | Self::Upstream(message) | Self::Unavailable(message) => {
                message.clone()
            }
            Self::InvalidCredentials => "Invalid email, username, or password".to_string(),
            Self::Unauthorized => "Authentication required".to_string(),
            Self::Forbidden => "You do not have permission to access this resource".to_string(),
            Self::NotFound => "Resource not found".to_string(),
            Self::EmailTaken => "An account with this email already exists".to_string(),
            Self::UsernameTaken => "An account with this username already exists".to_string(),
            Self::Internal(detail) => {
                eprintln!("error: {detail}");
                "An unexpected error occurred".to_string()
            }
        }
    }

    /// Maps a unique-constraint violation into the `taken` error for the
    /// offending column; any other database error becomes `Internal`.
    ///
    /// Shared by registration and admin edits, since both write to `users`.
    pub fn from_user_conflict(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                if db.message().contains("users.username") {
                    Self::UsernameTaken
                } else {
                    Self::EmailTaken
                }
            }
            other => Self::from(other),
        }
    }

    /// Maps a `servers` unique-constraint violation onto the field that caused
    /// it, so a race between two creates still produces a useful message.
    pub fn from_server_conflict(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                Self::Validation(vec![FieldError {
                    field: "hostPort".to_string(),
                    message: "That port is already used by another server".to_string(),
                }])
            }
            other => Self::from(other),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(format!("database error: {error}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, fields) = match self {
            Self::Validation(fields) => (
                StatusCode::BAD_REQUEST,
                "validation_error",
                "Validation failed".to_string(),
                fields,
            ),
            Self::Runtime(message) => (
                StatusCode::BAD_REQUEST,
                "runtime_error",
                message,
                Vec::new(),
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid email, username, or password".to_string(),
                Vec::new(),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required".to_string(),
                Vec::new(),
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "You do not have permission to access this resource".to_string(),
                Vec::new(),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "Resource not found".to_string(),
                Vec::new(),
            ),
            Self::EmailTaken => (
                StatusCode::CONFLICT,
                "email_taken",
                "An account with this email already exists".to_string(),
                Vec::new(),
            ),
            Self::UsernameTaken => (
                StatusCode::CONFLICT,
                "username_taken",
                "An account with this username already exists".to_string(),
                Vec::new(),
            ),
            Self::Upstream(message) => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                message,
                Vec::new(),
            ),
            Self::Unavailable(message) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime_unavailable",
                message,
                Vec::new(),
            ),
            Self::Internal(detail) => {
                eprintln!("error: {detail}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An unexpected error occurred".to_string(),
                    Vec::new(),
                )
            }
        };

        let mut error = serde_json::json!({ "code": code, "message": message });
        if !fields.is_empty() {
            error["fields"] = serde_json::to_value(fields).unwrap_or(serde_json::Value::Null);
        }

        (status, Json(serde_json::json!({ "error": error }))).into_response()
    }
}

/// Lets an `ApiError` propagate out of `main` (and `?` in constructors), where
/// the only thing worth printing is the message.
impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.reason())
    }
}

impl std::error::Error for ApiError {}
