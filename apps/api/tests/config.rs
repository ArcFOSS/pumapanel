//! Configuration validation tests.

use api::config::{Config, Environment};

#[test]
fn production_requires_jwt_secret() {
    let error = Config::load(|key| match key {
        "APP_ENV" => Some("production".to_string()),
        _ => None,
    })
    .expect_err("production must reject a missing JWT_SECRET");

    assert!(error.to_string().contains("JWT_SECRET"));
}

#[test]
fn rejects_too_short_secret() {
    let result = Config::load(|key| match key {
        "JWT_SECRET" => Some("too-short".to_string()),
        _ => None,
    });

    assert!(result.is_err(), "a weak secret must be rejected");
}

#[test]
fn development_generates_an_ephemeral_secret() {
    let config = Config::load(|_| None).expect("development config");

    assert_eq!(config.app_env, Environment::Development);
    assert!(config.jwt_secret.len() >= 32);
    // Development defaults to a non-Secure cookie so plain HTTP works.
    assert!(!config.cookie_secure);
}

#[test]
fn production_defaults_cookie_to_secure() {
    let config = Config::load(|key| match key {
        "APP_ENV" => Some("production".to_string()),
        "JWT_SECRET" => Some("a-sufficiently-long-test-secret-value".to_string()),
        _ => None,
    })
    .expect("production config");

    assert!(config.cookie_secure);
}

#[test]
fn explicit_values_override_defaults() {
    let config = Config::load(|key| match key {
        "APP_ENV" => Some("development".to_string()),
        "JWT_SECRET" => Some("explicit-test-secret-value-0123456789".to_string()),
        "COOKIE_SECURE" => Some("true".to_string()),
        "JWT_EXPIRATION_SECS" => Some("60".to_string()),
        "COOKIE_NAME" => Some("session".to_string()),
        _ => None,
    })
    .expect("config with overrides");

    assert!(config.cookie_secure);
    assert_eq!(config.jwt_expiration.as_secs(), 60);
    assert_eq!(config.cookie_name, "session");
}
