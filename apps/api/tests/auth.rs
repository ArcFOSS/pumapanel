//! Integration tests for the authentication flow.
//!
//! Each test runs against the real router and a throwaway SQLite database file,
//! so no external services or credentials are required.

use std::sync::Arc;

use api::auth::jwt::{self, Claims};
use api::config::Config;
use api::db;
use api::state::AppState;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const TEST_SECRET: &str = "test-secret-0123456789abcdef0123456789abcdef";
const COOKIE_NAME: &str = "pumapanel_token";

struct TestContext {
    app: Router,
    pool: SqlitePool,
    config: Arc<Config>,
}

async fn setup() -> TestContext {
    let db_path = std::env::temp_dir().join(format!("pumapanel-test-{}.db", uuid::Uuid::new_v4()));
    let pool = db::init_pool(&format!("sqlite://{}", db_path.display()))
        .await
        .expect("initialise test database");

    let config = Arc::new(
        Config::load(|key| match key {
            "JWT_SECRET" => Some(TEST_SECRET.to_string()),
            "APP_ENV" => Some("test".to_string()),
            _ => None,
        })
        .expect("build test configuration"),
    );

    let state = AppState {
        db: pool.clone(),
        config: Arc::clone(&config),
    };

    TestContext {
        app: api::app(state),
        pool,
        config,
    }
}

struct Response {
    status: StatusCode,
    body: String,
    cookies: Vec<String>,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

async fn send(app: &Router, request: Request<Body>) -> Response {
    let response = app.clone().oneshot(request).await.expect("send request");
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_string)
        .collect();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");

    Response {
        status,
        body: String::from_utf8_lossy(&body).to_string(),
        cookies,
    }
}

fn json_post(path: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build request")
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("build request")
}

fn with_header(
    mut request: Request<Body>,
    name: header::HeaderName,
    value: String,
) -> Request<Body> {
    request
        .headers_mut()
        .insert(name, value.parse().expect("valid header value"));
    request
}

/// Returns the full `Set-Cookie` header for the session cookie, if present.
fn full_session_cookie(cookies: &[String]) -> Option<String> {
    cookies
        .iter()
        .find(|cookie| cookie.starts_with(&format!("{COOKIE_NAME}=")))
        .cloned()
}

/// Returns the `name=value` pair of the session cookie, if present.
fn session_cookie(cookies: &[String]) -> Option<String> {
    full_session_cookie(cookies).and_then(|cookie| cookie.split(';').next().map(str::to_string))
}

fn token_from(cookies: &[String]) -> String {
    session_cookie(cookies)
        .expect("session cookie present")
        .trim_start_matches(&format!("{COOKIE_NAME}="))
        .to_string()
}

fn user_id(body: &Value) -> String {
    body["user"]["id"]
        .as_str()
        .expect("user id present")
        .to_string()
}

async fn register(app: &Router, email: &str, password: &str) -> Response {
    send(
        app,
        json_post(
            "/auth/register",
            json!({ "email": email, "password": password }),
        ),
    )
    .await
}

#[tokio::test]
async fn register_persists_account_and_authenticates() {
    let ctx = setup().await;

    let response = register(&ctx.app, "  New.User@Example.com ", "correct horse battery").await;

    assert_eq!(response.status, StatusCode::CREATED);
    // Email is normalised to lowercase and trimmed.
    assert_eq!(response.json()["user"]["email"], "new.user@example.com");
    assert!(response.json()["user"]["createdAt"].is_string());
    // Registration logs the user in immediately.
    assert!(session_cookie(&response.cookies).is_some());

    let stored = api::users::find_by_email(&ctx.pool, "new.user@example.com")
        .await
        .expect("query user")
        .expect("user was persisted");
    assert!(
        stored.password_hash.starts_with("$argon2id$"),
        "password must be stored as an Argon2id hash"
    );
    // The response never leaks the hash or the password field.
    assert!(!response.body.contains(&stored.password_hash));
    assert!(!response.body.to_lowercase().contains("password"));
}

#[tokio::test]
async fn register_rejects_duplicate_email_case_insensitively() {
    let ctx = setup().await;

    let first = register(&ctx.app, "dup@example.com", "password123").await;
    assert_eq!(first.status, StatusCode::CREATED);

    let second = register(&ctx.app, "DUP@example.com", "password456").await;
    assert_eq!(second.status, StatusCode::CONFLICT);
    assert_eq!(second.json()["error"]["code"], "email_taken");
}

#[tokio::test]
async fn register_rejects_invalid_email() {
    let ctx = setup().await;

    let response = register(&ctx.app, "not-an-email", "password123").await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["code"], "validation_error");
    assert_eq!(response.json()["error"]["fields"][0]["field"], "email");
}

#[tokio::test]
async fn register_rejects_short_password() {
    let ctx = setup().await;

    let response = register(&ctx.app, "short@example.com", "short").await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["fields"][0]["field"], "password");
}

#[tokio::test]
async fn login_accepts_valid_credentials() {
    let ctx = setup().await;
    register(&ctx.app, "login@example.com", "password123").await;

    let response = send(
        &ctx.app,
        json_post(
            "/auth/login",
            json!({ "email": "login@example.com", "password": "password123" }),
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.json()["user"]["id"].is_string());
    assert!(session_cookie(&response.cookies).is_some());
}

#[tokio::test]
async fn login_rejects_unknown_email_and_wrong_password_identically() {
    let ctx = setup().await;
    register(&ctx.app, "known@example.com", "password123").await;

    let wrong_password = send(
        &ctx.app,
        json_post(
            "/auth/login",
            json!({ "email": "known@example.com", "password": "wrong-password" }),
        ),
    )
    .await;
    let unknown_email = send(
        &ctx.app,
        json_post(
            "/auth/login",
            json!({ "email": "nobody@example.com", "password": "wrong-password" }),
        ),
    )
    .await;

    assert_eq!(wrong_password.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_email.status, StatusCode::UNAUTHORIZED);
    // Identical response so account existence is not revealed.
    assert_eq!(wrong_password.body, unknown_email.body);
    assert_eq!(
        wrong_password.json()["error"]["code"],
        "invalid_credentials"
    );
}

#[tokio::test]
async fn me_requires_authentication() {
    let ctx = setup().await;

    let response = send(&ctx.app, get("/auth/me")).await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(response.json()["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn me_returns_user_for_session_cookie() {
    let ctx = setup().await;
    let registered = register(&ctx.app, "cookie@example.com", "password123").await;
    let cookie = session_cookie(&registered.cookies).expect("session cookie");

    let response = send(
        &ctx.app,
        with_header(get("/auth/me"), header::COOKIE, cookie),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["user"]["id"], user_id(&registered.json()));
    assert_eq!(response.json()["user"]["email"], "cookie@example.com");
}

#[tokio::test]
async fn me_accepts_bearer_token() {
    let ctx = setup().await;
    let registered = register(&ctx.app, "bearer@example.com", "password123").await;
    let token = token_from(&registered.cookies);

    let response = send(
        &ctx.app,
        with_header(
            get("/auth/me"),
            header::AUTHORIZATION,
            format!("Bearer {token}"),
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["user"]["email"], "bearer@example.com");
}

#[tokio::test]
async fn me_rejects_token_signed_with_a_different_secret() {
    let ctx = setup().await;
    let registered = register(&ctx.app, "tamper@example.com", "password123").await;

    let claims = Claims {
        sub: user_id(&registered.json()),
        iss: "pumapanel".to_string(),
        iat: jwt::now_secs(),
        exp: jwt::now_secs() + 3600,
    };
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(b"a-totally-different-signing-secret"),
    )
    .expect("encode token");

    let response = send(
        &ctx.app,
        with_header(
            get("/auth/me"),
            header::AUTHORIZATION,
            format!("Bearer {token}"),
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_rejects_expired_token() {
    let ctx = setup().await;
    let registered = register(&ctx.app, "expired@example.com", "password123").await;

    let claims = Claims {
        sub: user_id(&registered.json()),
        iss: "pumapanel".to_string(),
        iat: 0,
        exp: 1,
    };
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(ctx.config.jwt_secret.as_bytes()),
    )
    .expect("encode token");

    let response = send(
        &ctx.app,
        with_header(
            get("/auth/me"),
            header::AUTHORIZATION,
            format!("Bearer {token}"),
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_clears_session_cookie() {
    let ctx = setup().await;
    let registered = register(&ctx.app, "logout@example.com", "password123").await;
    let cookie = session_cookie(&registered.cookies).expect("session cookie");

    let response = send(
        &ctx.app,
        with_header(json_post("/auth/logout", json!({})), header::COOKIE, cookie),
    )
    .await;

    assert_eq!(response.status, StatusCode::NO_CONTENT);
    let clearing = full_session_cookie(&response.cookies).expect("clearing cookie");
    let lower = clearing.to_lowercase();
    assert!(
        lower.contains("max-age=0") || lower.contains("expires="),
        "logout must expire the cookie, got: {clearing}"
    );
}

#[tokio::test]
async fn health_endpoint_still_works() {
    let ctx = setup().await;

    let response = send(&ctx.app, get("/health")).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["status"], "ok");
}
