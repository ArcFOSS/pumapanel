//! Admin-only user management.
//!
//! Routes are gated by [`AdminUser`], so only accounts whose *current* role is
//! `admin` (read from the database on every request) can reach them. The
//! frontend hides these pages from non-admins, but the API is the actual
//! security boundary: every handler re-authorizes, and non-admins receive
//! `403 Forbidden` without the response revealing whether an account exists.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use validator::ValidateEmail;

use crate::auth::extractor::AdminUser;
use crate::error::{ApiError, FieldError};
use crate::state::AppState;
use crate::users::{self, PublicUser, Role};

/// Admin routes. Mounted by the application router alongside `auth`.
pub fn router() -> Router<AppState> {
    Router::new().route("/admin/users/{id}", get(get_user).patch(update_user))
}

/// The fields an administrator may edit on a user account.
///
/// Every field is optional so a client can send a partial update.
#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    username: Option<String>,
    email: Option<String>,
    role: Option<Role>,
}

#[derive(serde::Serialize)]
struct UserResponse {
    user: PublicUser,
}

/// Returns a user's profile. Admin only.
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _admin: AdminUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user = users::find_by_id(&state.db, &id)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(UserResponse { user: user.into() }))
}

/// Applies an administrator's edits to a user account. Admin only.
///
/// Username and email use the same rules as registration. An administrator can
/// change anyone's role — user to admin, or the reverse — but never their own:
/// that is the route by which a lone admin could demote themselves and lock
/// everyone out of the admin API.
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    AdminUser(admin): AdminUser,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let current = users::find_by_id(&state.db, &id)
        .await?
        .ok_or(ApiError::NotFound)?;

    let username = payload.username.as_deref().map(users::normalize_username);
    let email = payload.email.as_deref().map(users::normalize_email);
    let role = payload.role;

    // Validate every provided field with the same rules as registration.
    let mut fields: Vec<FieldError> = Vec::new();
    if let Some(username) = &username
        && (username.chars().count() < 3 || username.chars().count() > 30)
    {
        fields.push(FieldError {
            field: "username".to_string(),
            message: "Username must be between 3 and 30 characters".to_string(),
        });
    }
    if let Some(username) = &username
        && !users::has_valid_username_chars(username)
    {
        fields.push(FieldError {
            field: "username".to_string(),
            message: "Username may only contain letters, numbers, and underscores".to_string(),
        });
    }
    if let Some(email) = &email
        && !email.validate_email()
    {
        fields.push(FieldError {
            field: "email".to_string(),
            message: "Enter a valid email address".to_string(),
        });
    }
    if !fields.is_empty() {
        return Err(ApiError::Validation(fields));
    }

    if let Some(role) = role
        && admin.id == current.id
        && role != current.role
    {
        return Err(ApiError::Forbidden);
    }

    let updated = users::update(&state.db, &current.id, username, email, role)
        .await
        .map_err(ApiError::from_user_conflict)?;

    Ok(Json(UserResponse { user: updated.into() }))
}