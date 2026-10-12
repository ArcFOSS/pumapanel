//! Integration tests for the authentication flow.
//!
//! Each test runs against the real router and a throwaway SQLite database file,
//! so no external services or credentials are required.

mod common;

use api::auth::jwt::{self, Claims};
use axum::http::{StatusCode, header};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::json;

use common::*;

#[tokio::test]
async fn register_persists_account_and_authenticates() {
    let ctx = setup().await;

    let response = register(
        &ctx.app,
        "  NewUser  ",
        "  New.User@Example.com ",
        "correct horse battery",
    )
    .await;

    assert_eq!(response.status, StatusCode::CREATED);
    // Username and email are normalized to lowercase and trimmed.
    assert_eq!(response.json()["user"]["username"], "newuser");
    assert_eq!(response.json()["user"]["email"], "new.user@example.com");
    assert!(response.json()["user"]["createdAt"].is_string());
    // Registration logs the user in immediately.
    assert!(session_cookie(&response.cookies).is_some());

    let stored = api::users::find_by_email(&ctx.pool, "new.user@example.com")
        .await
        .expect("query user")
        .expect("user was persisted");
    assert_eq!(stored.username, "newuser");
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

    let first = register(&ctx.app, "first", "dup@example.com", "password123").await;
    assert_eq!(first.status, StatusCode::CREATED);

    let second = register(&ctx.app, "second", "DUP@example.com", "password456").await;
    assert_eq!(second.status, StatusCode::CONFLICT);
    assert_eq!(second.json()["error"]["code"], "email_taken");
}

#[tokio::test]
async fn register_rejects_duplicate_username_case_insensitively() {
    let ctx = setup().await;

    let first = register(&ctx.app, "taken", "first@example.com", "password123").await;
    assert_eq!(first.status, StatusCode::CREATED);

    let second = register(&ctx.app, "TAKEN", "second@example.com", "password456").await;
    assert_eq!(second.status, StatusCode::CONFLICT);
    assert_eq!(second.json()["error"]["code"], "username_taken");
}

#[tokio::test]
async fn register_rejects_malformed_username() {
    let ctx = setup().await;

    // Too short.
    let short = register(&ctx.app, "ab", "short@example.com", "password123").await;
    assert_eq!(short.status, StatusCode::BAD_REQUEST);
    assert_eq!(short.json()["error"]["code"], "validation_error");
    assert_eq!(short.json()["error"]["fields"][0]["field"], "username");

    // Disallowed characters.
    let invalid = register(&ctx.app, "bad name!", "invalid@example.com", "password123").await;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid.json()["error"]["fields"][0]["field"], "username");
}

#[tokio::test]
async fn register_rejects_invalid_email() {
    let ctx = setup().await;

    let response = register(&ctx.app, "validname", "not-an-email", "password123").await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["code"], "validation_error");
    assert_eq!(response.json()["error"]["fields"][0]["field"], "email");
}

#[tokio::test]
async fn register_rejects_short_password() {
    let ctx = setup().await;

    let response = register(&ctx.app, "validname", "short@example.com", "short").await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["fields"][0]["field"], "password");
}

#[tokio::test]
async fn login_accepts_email_credentials() {
    let ctx = setup().await;
    register(&ctx.app, "loginuser", "login@example.com", "password123").await;

    let response = login(&ctx.app, "login@example.com", "password123").await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.json()["user"]["id"].is_string());
    assert!(session_cookie(&response.cookies).is_some());
}

#[tokio::test]
async fn login_accepts_username_case_insensitively() {
    let ctx = setup().await;
    register(&ctx.app, "loginuser", "login@example.com", "password123").await;

    let response = login(&ctx.app, "  LoginUser  ", "password123").await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["user"]["username"], "loginuser");
}

#[tokio::test]
async fn login_rejects_unknown_identifier_and_wrong_password_identically() {
    let ctx = setup().await;
    register(&ctx.app, "known", "known@example.com", "password123").await;

    let wrong_password = login(&ctx.app, "known@example.com", "wrong-password").await;
    let unknown_identifier = login(&ctx.app, "nobody@example.com", "wrong-password").await;

    assert_eq!(wrong_password.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_identifier.status, StatusCode::UNAUTHORIZED);
    // Identical response so account existence is not revealed.
    assert_eq!(wrong_password.body, unknown_identifier.body);
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
    let registered = register(&ctx.app, "cookieuser", "cookie@example.com", "password123").await;
    let cookie = session_cookie(&registered.cookies).expect("session cookie");

    let response = send(
        &ctx.app,
        with_header(get("/auth/me"), header::COOKIE, cookie),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["user"]["id"], user_id(&registered.json()));
    assert_eq!(response.json()["user"]["email"], "cookie@example.com");
    assert_eq!(response.json()["user"]["username"], "cookieuser");
}

#[tokio::test]
async fn me_accepts_bearer_token() {
    let ctx = setup().await;
    let registered = register(&ctx.app, "beareruser", "bearer@example.com", "password123").await;
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
    let registered = register(&ctx.app, "tamperuser", "tamper@example.com", "password123").await;

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
    let registered = register(&ctx.app, "expireduser", "expired@example.com", "password123").await;

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
    let registered = register(&ctx.app, "logoutuser", "logout@example.com", "password123").await;
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
