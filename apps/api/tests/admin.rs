//! Integration tests for the admin user-management API and RBAC.
//!
//! Admin routes sit behind the `AdminUser` extractor: a request without a valid
//! session is `401`, and an authenticated account whose current role is not
//! `admin` is `403` — before any handler logic runs.

mod common;

use axum::http::{StatusCode, header};
use serde_json::json;
use sqlx::SqlitePool;

use common::*;

const PASSWORD: &str = "password123";

/// Registers an account and returns its session cookie as a `name=value` pair.
async fn session_for(ctx: &TestContext, username: &str, email: &str) -> String {
    let registered = register(&ctx.app, username, email, PASSWORD).await;
    assert_eq!(registered.status, StatusCode::CREATED);
    session_cookie(&registered.cookies).expect("session cookie")
}

/// Promotes an existing account to `admin` via the database.
///
/// The role is re-read from the database on every request, so the account's
/// existing session cookie immediately grants admin access — no re-login.
async fn promote_to_admin(pool: &SqlitePool, id: &str) {
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .expect("promote user to admin");
}

/// Registers an account, promotes it to admin, and returns its session cookie.
async fn admin_session(ctx: &TestContext, username: &str, email: &str) -> String {
    let cookie = session_for(ctx, username, email).await;
    let me = send(&ctx.app, with_header(get("/auth/me"), header::COOKIE, cookie.clone())).await;
    assert_eq!(me.status, StatusCode::OK);
    promote_to_admin(&ctx.pool, &user_id(&me.json())).await;
    cookie
}

#[tokio::test]
async fn admin_endpoints_require_authentication() {
    let ctx = setup().await;

    let get_response = send(&ctx.app, get("/admin/users/missing-user")).await;
    let patch_response = send(
        &ctx.app,
        patch("/admin/users/missing-user", json!({ "role": "admin" })),
    )
    .await;

    assert_eq!(get_response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(get_response.json()["error"]["code"], "unauthorized");
    assert_eq!(patch_response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(patch_response.json()["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn admin_endpoints_reject_non_admin_users() {
    let ctx = setup().await;
    let cookie = session_for(&ctx, "plainuser", "plain@example.com").await;
    let target = register(&ctx.app, "target", "target@example.com", PASSWORD).await;
    let target_id = user_id(&target.json());
    let path = format!("/admin/users/{target_id}");

    // The account exists, but the caller is not an admin — the response must
    // not reveal anything beyond "forbidden".
    let get_response = send(
        &ctx.app,
        with_header(get(&path), header::COOKIE, cookie.clone()),
    )
    .await;
    let patch_response = send(
        &ctx.app,
        with_header(
            patch(&path, json!({ "email": "hijacked@example.com" })),
            header::COOKIE,
            cookie,
        ),
    )
    .await;

    assert_eq!(get_response.status, StatusCode::FORBIDDEN);
    assert_eq!(get_response.json()["error"]["code"], "forbidden");
    assert!(get_response.json()["error"].get("fields").is_none());
    assert_eq!(patch_response.status, StatusCode::FORBIDDEN);

    // Neither request changed the target.
    let stored = api::users::find_by_id(&ctx.pool, &target_id)
        .await
        .expect("query user")
        .expect("user exists");
    assert_eq!(stored.email, "target@example.com");
    assert_eq!(stored.role, api::users::Role::User);
}

#[tokio::test]
async fn me_exposes_the_account_role() {
    let ctx = setup().await;
    let cookie = session_for(&ctx, "roleuser", "role@example.com").await;

    let before = send(&ctx.app, with_header(get("/auth/me"), header::COOKIE, cookie.clone())).await;
    assert_eq!(before.status, StatusCode::OK);
    assert_eq!(before.json()["user"]["role"], "user");

    promote_to_admin(&ctx.pool, &user_id(&before.json())).await;

    // Role changes are live for existing sessions.
    let after = send(&ctx.app, with_header(get("/auth/me"), header::COOKIE, cookie)).await;
    assert_eq!(after.json()["user"]["role"], "admin");
}

#[tokio::test]
async fn admin_can_view_any_user() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "adminuser", "admin@example.com").await;
    let target = register(&ctx.app, "viewtarget", "viewtarget@example.com", PASSWORD).await;
    let target_id = user_id(&target.json());

    let response = send(
        &ctx.app,
        with_header(
            get(&format!("/admin/users/{target_id}")),
            header::COOKIE,
            admin_cookie,
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["user"]["id"], target_id);
    assert_eq!(response.json()["user"]["username"], "viewtarget");
    assert_eq!(response.json()["user"]["email"], "viewtarget@example.com");
    assert_eq!(response.json()["user"]["role"], "user");
    assert!(response.json()["user"]["createdAt"].is_string());
    // The password hash never leaves the backend.
    assert!(!response.body.contains("password"));
}

#[tokio::test]
async fn get_user_returns_404_for_unknown_id() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "admin404", "admin404@example.com").await;

    let response = send(
        &ctx.app,
        with_header(get("/admin/users/missing-user"), header::COOKIE, admin_cookie),
    )
    .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.json()["error"]["code"], "not_found");
}

#[tokio::test]
async fn admin_can_edit_a_users_contact_details() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "editor", "editor@example.com").await;
    let target = register(&ctx.app, "edittarget", "edit@example.com", PASSWORD).await;
    let target_id = user_id(&target.json());

    let response = send(
        &ctx.app,
        with_header(
            patch(
                &format!("/admin/users/{target_id}"),
                json!({ "username": "renamed", "email": "renamed@example.com" }),
            ),
            header::COOKIE,
            admin_cookie,
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["user"]["username"], "renamed");
    assert_eq!(response.json()["user"]["email"], "renamed@example.com");
    assert_eq!(response.json()["user"]["role"], "user");

    // The new username is live and the old email no longer identifies the
    // account.
    let by_username = login(&ctx.app, "renamed", PASSWORD).await;
    assert_eq!(by_username.status, StatusCode::OK);
    let by_old_email = login(&ctx.app, "edit@example.com", PASSWORD).await;
    assert_eq!(by_old_email.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_can_change_a_users_role() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "promoter", "promoter@example.com").await;
    let target = register(&ctx.app, "promotee", "promotee@example.com", PASSWORD).await;
    let target_id = user_id(&target.json());
    let target_cookie = session_cookie(&target.cookies).expect("session cookie");
    let path = format!("/admin/users/{target_id}");

    // The target starts as a regular user.
    let before = send(&ctx.app, with_header(get(&path), header::COOKIE, target_cookie.clone())).await;
    assert_eq!(before.status, StatusCode::FORBIDDEN);

    // Promote to admin.
    let promote = send(
        &ctx.app,
        with_header(patch(&path, json!({ "role": "admin" })), header::COOKIE, admin_cookie.clone()),
    )
    .await;
    assert_eq!(promote.status, StatusCode::OK);
    assert_eq!(promote.json()["user"]["role"], "admin");

    // The promotion is effective immediately for the existing session.
    let after = send(&ctx.app, with_header(get(&path), header::COOKIE, target_cookie.clone())).await;
    assert_eq!(after.status, StatusCode::OK);

    // Demote again; admin access is revoked at the next request.
    let demote = send(
        &ctx.app,
        with_header(patch(&path, json!({ "role": "user" })), header::COOKIE, admin_cookie),
    )
    .await;
    assert_eq!(demote.status, StatusCode::OK);
    assert_eq!(demote.json()["user"]["role"], "user");

    let revoked = send(&ctx.app, with_header(get(&path), header::COOKIE, target_cookie)).await;
    assert_eq!(revoked.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn update_user_validates_provided_fields() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "validator", "validator@example.com").await;
    let target = register(&ctx.app, "validtarget", "validtarget@example.com", PASSWORD).await;
    let target_id = user_id(&target.json());
    let path = format!("/admin/users/{target_id}");

    let short_username = send(
        &ctx.app,
        with_header(patch(&path, json!({ "username": "ab" })), header::COOKIE, admin_cookie.clone()),
    )
    .await;
    assert_eq!(short_username.status, StatusCode::BAD_REQUEST);
    assert_eq!(short_username.json()["error"]["code"], "validation_error");
    assert_eq!(short_username.json()["error"]["fields"][0]["field"], "username");

    let invalid_chars = send(
        &ctx.app,
        with_header(patch(&path, json!({ "username": "bad name!" })), header::COOKIE, admin_cookie.clone()),
    )
    .await;
    assert_eq!(invalid_chars.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid_chars.json()["error"]["fields"][0]["field"], "username");

    let invalid_email = send(
        &ctx.app,
        with_header(patch(&path, json!({ "email": "not-an-email" })), header::COOKIE, admin_cookie),
    )
    .await;
    assert_eq!(invalid_email.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid_email.json()["error"]["fields"][0]["field"], "email");
}

#[tokio::test]
async fn update_user_rejects_duplicate_email_and_username() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "duper", "duper@example.com").await;
    register(&ctx.app, "first", "first@example.com", PASSWORD).await;
    let second = register(&ctx.app, "second", "second@example.com", PASSWORD).await;
    let second_id = user_id(&second.json());
    let path = format!("/admin/users/{second_id}");

    // Username collisions collide with the *first* account's username.
    let duplicate_username = send(
        &ctx.app,
        with_header(
            patch(&path, json!({ "username": "first" })),
            header::COOKIE,
            admin_cookie.clone(),
        ),
    )
    .await;
    assert_eq!(duplicate_username.status, StatusCode::CONFLICT);
    assert_eq!(duplicate_username.json()["error"]["code"], "username_taken");

    let duplicate_email = send(
        &ctx.app,
        with_header(
            patch(&path, json!({ "email": "first@example.com" })),
            header::COOKIE,
            admin_cookie,
        ),
    )
    .await;
    assert_eq!(duplicate_email.status, StatusCode::CONFLICT);
    assert_eq!(duplicate_email.json()["error"]["code"], "email_taken");
}

#[tokio::test]
async fn admin_cannot_change_their_own_role() {
    let ctx = setup().await;
    let cookie = admin_session(&ctx, "selfrole", "selfrole@example.com").await;
    let me = send(&ctx.app, with_header(get("/auth/me"), header::COOKIE, cookie.clone())).await;
    let my_id = user_id(&me.json());
    let path = format!("/admin/users/{my_id}");

    // Demoting themselves is rejected outright.
    let demote = send(
        &ctx.app,
        with_header(patch(&path, json!({ "role": "user" })), header::COOKIE, cookie.clone()),
    )
    .await;
    assert_eq!(demote.status, StatusCode::FORBIDDEN);
    assert_eq!(demote.json()["error"]["code"], "forbidden");

    // Stating their own role (a no-op) and editing their contact details are
    // both allowed.
    let keep = send(
        &ctx.app,
        with_header(patch(&path, json!({ "role": "admin" })), header::COOKIE, cookie.clone()),
    )
    .await;
    assert_eq!(keep.status, StatusCode::OK);

    let rename = send(
        &ctx.app,
        with_header(patch(&path, json!({ "username": "selfrenamed" })), header::COOKIE, cookie),
    )
    .await;
    assert_eq!(rename.status, StatusCode::OK);
    assert_eq!(rename.json()["user"]["username"], "selfrenamed");
}

#[tokio::test]
async fn update_user_returns_404_for_unknown_id() {
    let ctx = setup().await;
    let admin_cookie = admin_session(&ctx, "adminupd404", "admin@update-404.com").await;

    let response = send(
        &ctx.app,
        with_header(
            patch("/admin/users/missing-user", json!({ "username": "newname" })),
            header::COOKIE,
            admin_cookie,
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.json()["error"]["code"], "not_found");
}