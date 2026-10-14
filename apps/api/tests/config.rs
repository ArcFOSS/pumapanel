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

#[test]
fn server_management_is_disabled_without_an_agent() {
    let config = Config::load(|_| None).expect("development config");

    assert!(config.agent_url.is_none());
    // The PaperMC catalogue is still configured, so the versions endpoint works
    // even when no container can be started.
    assert!(config.paper_api_url.starts_with("https://"));
    assert_eq!(config.default_server_memory_mb, 2048);
}

#[test]
fn agent_url_requires_a_token() {
    let error = Config::load(|key| match key {
        "AGENT_URL" => Some("http://127.0.0.1:9091".to_string()),
        _ => None,
    })
    .expect_err("an agent without a token must be rejected");

    assert!(error.to_string().contains("AGENT_TOKEN"));
}

#[test]
fn accepts_a_fully_configured_agent() {
    let config = Config::load(|key| match key {
        "AGENT_URL" => Some("http://agent:9091".to_string()),
        "AGENT_TOKEN" => Some("0123456789abcdef".to_string()),
        "AGENT_TIMEOUT_SECS" => Some("120".to_string()),
        "PAPER_API_URL" => Some("http://paper.test/v3".to_string()),
        _ => None,
    })
    .expect("agent config");

    assert_eq!(config.agent_url.as_deref(), Some("http://agent:9091"));
    assert_eq!(config.agent_token.as_deref(), Some("0123456789abcdef"));
    assert_eq!(config.agent_timeout.as_secs(), 120);
    assert_eq!(config.paper_api_url, "http://paper.test/v3");
}

#[test]
fn rejects_non_positive_timeouts() {
    let result = Config::load(|key| match key {
        "AGENT_TIMEOUT_SECS" => Some("0".to_string()),
        _ => None,
    });

    assert!(result.is_err(), "a zero timeout must be rejected");
}
