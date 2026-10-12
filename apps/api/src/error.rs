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
    /// 500 — unexpected server-side failure.
    Internal(String),
}

impl ApiError {
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal(detail.into())
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
