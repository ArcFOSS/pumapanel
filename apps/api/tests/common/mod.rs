//! Shared helpers for API integration tests.
//!
//! Cargo compiles each file in `tests/` as its own crate, so reusable setup
//! lives in a `common` module that every suite pulls in with
//! `mod common; use common::*;`.

#![allow(dead_code)] // each suite only exercises the helpers it needs

use std::sync::Arc;

use api::config::Config;
use api::db;
use api::state::AppState;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

pub const TEST_SECRET: &str = "test-secret-0123456789abcdef0123456789abcdef";
pub const COOKIE_NAME: &str = "pumapanel_token";

pub struct TestContext {
    pub app: Router,
    pub pool: SqlitePool,
    pub config: Arc<Config>,
}

/// Builds the real application router against a throwaway SQLite database.
pub async fn setup() -> TestContext {
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

/// A captured HTTP response enough for assertions: status, body, and cookies.
pub struct Response {
    pub status: StatusCode,
    pub body: String,
    pub cookies: Vec<String>,
}

impl Response {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

pub async fn send(app: &Router, request: Request<Body>) -> Response {
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

pub fn json_post(path: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build request")
}

pub fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("build request")
}

pub fn patch(path: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build request")
}

pub fn with_header(
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
pub fn full_session_cookie(cookies: &[String]) -> Option<String> {
    cookies
        .iter()
        .find(|cookie| cookie.starts_with(&format!("{COOKIE_NAME}=")))
        .cloned()
}

/// Returns the `name=value` pair of the session cookie, if present.
pub fn session_cookie(cookies: &[String]) -> Option<String> {
    full_session_cookie(cookies).and_then(|cookie| cookie.split(';').next().map(str::to_string))
}

pub fn token_from(cookies: &[String]) -> String {
    session_cookie(cookies)
        .expect("session cookie present")
        .trim_start_matches(&format!("{COOKIE_NAME}="))
        .to_string()
}

pub fn user_id(body: &Value) -> String {
    body["user"]["id"]
        .as_str()
        .expect("user id present")
        .to_string()
}

pub async fn register(app: &Router, username: &str, email: &str, password: &str) -> Response {
    send(
        app,
        json_post(
            "/auth/register",
            json!({ "username": username, "email": email, "password": password }),
        ),
    )
    .await
}

pub async fn login(app: &Router, identifier: &str, password: &str) -> Response {
    send(
        app,
        json_post(
            "/auth/login",
            json!({ "identifier": identifier, "password": password }),
        ),
    )
    .await
}