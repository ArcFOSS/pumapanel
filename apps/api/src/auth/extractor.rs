use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;

use crate::auth::jwt;
use crate::error::ApiError;
use crate::state::AppState;
use crate::users::{self, User};

/// Extractor that resolves the authenticated [`User`] for a request.
///
/// The token is read from the session cookie (preferred) or, failing that, an
/// `Authorization: Bearer <token>` header (useful for API clients/tests). Any
/// missing, malformed, expired, or invalid token — or a token whose user no
/// longer exists — is rejected with a generic `401`.
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token =
            extract_token(parts, &state.config.cookie_name).ok_or(ApiError::Unauthorized)?;

        let claims = jwt::verify_token(&state.config, &token)?;

        let user = users::find_by_id(&state.db, &claims.sub)
            .await?
            .ok_or(ApiError::Unauthorized)?;

        Ok(Self(user))
    }
}

/// Extractor that resolves an authenticated **admin** [`User`].
///
/// The role is read from the database on every request — the token only
/// identifies the account — so a role change takes effect immediately for
/// existing sessions. Authenticated non-admin users are rejected with `403`
/// regardless of which resource they target, so admin routes leak nothing to
/// unauthorized callers.
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        if !user.role.is_admin() {
            return Err(ApiError::Forbidden);
        }
        Ok(Self(user))
    }
}

fn extract_token(parts: &Parts, cookie_name: &str) -> Option<String> {
    if let Some(token) = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    let jar = CookieJar::from_headers(&parts.headers);
    jar.get(cookie_name)
        .map(|cookie| cookie.value().to_string())
        .filter(|token| !token.is_empty())
}
