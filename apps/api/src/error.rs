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
    /// 409 — an account with this email already exists.
    EmailTaken,
    /// 500 — unexpected server-side failure.
    Internal(String),
}

impl ApiError {
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal(detail.into())
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
                "Invalid email or password".to_string(),
                Vec::new(),
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Authentication required".to_string(),
                Vec::new(),
            ),
            Self::EmailTaken => (
                StatusCode::CONFLICT,
                "email_taken",
                "An account with this email already exists".to_string(),
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
