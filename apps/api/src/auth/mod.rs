pub mod extractor;
pub mod jwt;
pub mod password;

use std::sync::OnceLock;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration as CookieDuration;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError, ValidationErrors};

use crate::config::Config;
use crate::error::{ApiError, FieldError};
use crate::state::AppState;
use crate::users::{self, PublicUser};

use self::extractor::AuthUser;

/// Authentication routes. Mounted by the application router.
///
/// The production reverse proxy serves these under `/api` (stripping the
/// prefix), and the Next.js dev server rewrites `/api/*` to the API, so the
/// browser always talks to them same-origin.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// An email address or a username.
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: PublicUser,
}

#[derive(Debug, Validate)]
struct RegisterForm {
    #[validate(
        length(
            min = 3,
            max = 30,
            message = "Username must be between 3 and 30 characters"
        ),
        custom(
            function = "validate_username_chars",
            message = "Username may only contain letters, numbers, and underscores"
        )
    )]
    username: String,
    #[validate(email(message = "Enter a valid email address"))]
    email: String,
    #[validate(length(
        min = 8,
        max = 128,
        message = "Password must be between 8 and 128 characters"
    ))]
    password: String,
}

#[derive(Debug, Validate)]
struct LoginForm {
    #[validate(length(
        min = 1,
        max = 320,
        message = "Enter your email or username"
    ))]
    identifier: String,
    #[validate(length(min = 1, max = 128, message = "Password is required"))]
    password: String,
}

/// Restricts usernames to letters, digits, and underscores. Input is
/// normalized to lowercase before validation, so this also decides the stored
/// form. The same policy is enforced when an admin edits a username
/// (`[crate::admin]`).
fn validate_username_chars(username: &str) -> Result<(), ValidationError> {
    if users::has_valid_username_chars(username) {
        Ok(())
    } else {
        Err(ValidationError::new("username_chars"))
    }
}

/// Creates an account, persists it, and signs the new user in immediately by
/// issuing a session cookie.
async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<AuthResponse>), ApiError> {
    let form = RegisterForm {
        username: users::normalize_username(&payload.username),
        email: users::normalize_email(&payload.email),
        password: payload.password,
    };
    form.validate().map_err(validation_error)?;

    let password_hash = password::hash_password(&form.password)
        .map_err(|error| ApiError::internal(format!("failed to hash password: {error}")))?;

    let user = match users::create(&state.db, &form.username, &form.email, &password_hash).await {
        // The UNIQUE constraints are the authority, so concurrent registration
        // attempts cannot produce duplicate accounts. The offending column is
        // named in the SQLite error, which distinguishes the two conflicts.
        Ok(user) => user,
        Err(error) => return Err(ApiError::from_user_conflict(error)),
    };

    let token = jwt::issue_token(&state.config, &user.id)?;
    let jar = set_session_cookie(jar, &state.config, token);

    Ok((
        StatusCode::CREATED,
        jar,
        Json(AuthResponse { user: user.into() }),
    ))
}

/// Verifies credentials and issues a session cookie.
async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    let form = LoginForm {
        identifier: users::normalize_identifier(&payload.identifier),
        password: payload.password,
    };
    form.validate().map_err(validation_error)?;

    let user = match users::find_by_identifier(&state.db, &form.identifier).await? {
        Some(user) if password::verify_password(&form.password, &user.password_hash) => user,
        Some(_) => return Err(ApiError::InvalidCredentials),
        None => {
            // Spend a comparable amount of time hashing so that a missing
            // account is not distinguishable from a wrong password by timing.
            let _ = password::verify_password(&form.password, dummy_password_hash());
            return Err(ApiError::InvalidCredentials);
        }
    };

    let token = jwt::issue_token(&state.config, &user.id)?;
    let jar = set_session_cookie(jar, &state.config, token);

    Ok((jar, Json(AuthResponse { user: user.into() })))
}

/// Clears the session cookie.
///
/// JWTs are stateless, so this only removes the cookie from the browser; a
/// previously issued token remains valid until it expires (see the docs for
/// this limitation).
async fn logout(State(state): State<AppState>, jar: CookieJar) -> (StatusCode, CookieJar) {
    (
        StatusCode::NO_CONTENT,
        clear_session_cookie(jar, &state.config),
    )
}

/// Returns the currently authenticated user.
async fn me(AuthUser(user): AuthUser) -> Json<AuthResponse> {
    Json(AuthResponse { user: user.into() })
}

fn set_session_cookie(jar: CookieJar, config: &Config, token: String) -> CookieJar {
    let mut cookie = Cookie::new(config.cookie_name.clone(), token);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(config.cookie_secure);
    cookie.set_path("/");
    cookie.set_max_age(CookieDuration::seconds(
        i64::try_from(config.jwt_expiration.as_secs()).unwrap_or(i64::MAX),
    ));
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    jar.add(cookie)
}

fn clear_session_cookie(jar: CookieJar, config: &Config) -> CookieJar {
    let mut cookie = Cookie::new(config.cookie_name.clone(), "");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(config.cookie_secure);
    cookie.set_path("/");
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    // `remove` marks the cookie for removal (empty value, expired max-age)
    // while preserving the name, path, and domain so the browser matches it.
    jar.remove(cookie)
}

/// A stable Argon2id hash used only to equalise login timing for unknown users.
fn dummy_password_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        password::hash_password("*invalid-placeholder-password*").unwrap_or_else(|_| String::new())
    })
}

fn validation_error(errors: ValidationErrors) -> ApiError {
    let mut fields: Vec<FieldError> = errors
        .field_errors()
        .into_iter()
        .map(|(field, field_errors)| {
            let message = field_errors
                .iter()
                .find_map(|error| error.message.as_ref())
                .map_or_else(|| format!("Invalid {field}"), |message| message.to_string());
            FieldError {
                field: field.to_string(),
                message,
            }
        })
        .collect();
    fields.sort_by(|left, right| left.field.cmp(&right.field));
    ApiError::Validation(fields)
}
