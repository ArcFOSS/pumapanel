use std::time::Duration;

use uuid::Uuid;

/// Minimum length accepted for an explicit `JWT_SECRET`.
const MIN_SECRET_LEN: usize = 32;
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "3001";
const DEFAULT_DATABASE_URL: &str = "sqlite://pumapanel.db";
const DEFAULT_JWT_ISSUER: &str = "pumapanel";
const DEFAULT_COOKIE_NAME: &str = "pumapanel_token";
/// 24 hours.
const DEFAULT_JWT_EXPIRATION_SECS: u64 = 86_400;

/// Deployment environment. Controls whether insecure development fallbacks are
/// allowed and whether the session cookie defaults to `Secure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    fn parse(raw: Option<String>) -> Self {
        match raw
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("production" | "prod") => Self::Production,
            _ => Self::Development,
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

/// Invalid or missing configuration. The message is safe to print at startup.
#[derive(Debug)]
pub struct ConfigError(String);

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "configuration error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

/// Runtime configuration, resolved from the environment once at startup.
#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: Duration,
    pub jwt_issuer: String,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
    pub app_env: Environment,
}

impl Config {
    /// Loads configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::load(|key| std::env::var(key).ok())
    }

    /// Loads configuration from an arbitrary source. Kept generic so tests can
    /// exercise validation without mutating global process state.
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let app_env = Environment::parse(get("APP_ENV"));

        let jwt_secret = match non_empty(get("JWT_SECRET")) {
            Some(secret) => {
                if secret.len() < MIN_SECRET_LEN {
                    return Err(ConfigError::new(format!(
                        "JWT_SECRET must be at least {MIN_SECRET_LEN} characters"
                    )));
                }
                secret
            }
            // Never fall back to a weak, shared secret in production.
            None if app_env.is_production() => {
                return Err(ConfigError::new(
                    "JWT_SECRET is required when APP_ENV=production",
                ));
            }
            // Development convenience: an ephemeral random secret is generated
            // (never committed), at the cost of invalidating sessions on restart.
            None => {
                eprintln!(
                    "warning: JWT_SECRET is not set; generated an ephemeral development secret. \
                     Existing sessions are invalidated when the process restarts."
                );
                format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
            }
        };

        let jwt_expiration_secs = match non_empty(get("JWT_EXPIRATION_SECS")) {
            Some(raw) => raw
                .parse::<u64>()
                .ok()
                .filter(|secs| *secs > 0)
                .ok_or_else(|| {
                    ConfigError::new("JWT_EXPIRATION_SECS must be a positive integer")
                })?,
            None => DEFAULT_JWT_EXPIRATION_SECS,
        };

        let cookie_secure = match non_empty(get("COOKIE_SECURE")) {
            Some(raw) => parse_bool(&raw)
                .ok_or_else(|| ConfigError::new("COOKIE_SECURE must be `true` or `false`"))?,
            None => app_env.is_production(),
        };

        Ok(Self {
            host: non_empty(get("HOST")).unwrap_or_else(|| DEFAULT_HOST.to_string()),
            port: non_empty(get("PORT")).unwrap_or_else(|| DEFAULT_PORT.to_string()),
            database_url: non_empty(get("DATABASE_URL"))
                .unwrap_or_else(|| DEFAULT_DATABASE_URL.to_string()),
            jwt_secret,
            jwt_expiration: Duration::from_secs(jwt_expiration_secs),
            jwt_issuer: non_empty(get("JWT_ISSUER"))
                .unwrap_or_else(|| DEFAULT_JWT_ISSUER.to_string()),
            cookie_name: non_empty(get("COOKIE_NAME"))
                .unwrap_or_else(|| DEFAULT_COOKIE_NAME.to_string()),
            cookie_secure,
            cookie_domain: non_empty(get("COOKIE_DOMAIN")),
            app_env,
        })
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}
