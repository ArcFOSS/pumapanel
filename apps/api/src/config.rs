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
/// PaperMC's Fill API (v3). The older `api.papermc.io/v2` endpoints are gone.
const DEFAULT_PAPER_API_URL: &str = "https://fill.papermc.io/v3";
/// Creating a server can pull the Minecraft image, which is slow. The agent
/// therefore gets a much longer deadline than a typical request.
const DEFAULT_AGENT_TIMEOUT_SECS: u64 = 600;
const DEFAULT_PAPER_TIMEOUT_SECS: u64 = 15;
const DEFAULT_PAPER_CACHE_TTL_SECS: u64 = 3_600;
const DEFAULT_SERVER_MEMORY_MB: u32 = 2_048;

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
    /// Base URL of the PumaPanel agent. `None` disables server management: the
    /// panel still runs, but it cannot start or list servers. Development
    /// (`moon run dev`) leaves it unset.
    pub agent_url: Option<String>,
    /// Shared secret for the agent. Required whenever `agent_url` is set.
    pub agent_token: Option<String>,
    pub agent_timeout: Duration,
    /// Base URL of the PaperMC version API, proxied so the browser never talks
    /// to PaperMC directly (and can be pointed at a stub in tests).
    pub paper_api_url: String,
    pub paper_timeout: Duration,
    /// How long the PaperMC version catalogue is served from memory.
    pub paper_cache_ttl: Duration,
    /// JVM heap given to a new server when the request does not choose one.
    pub default_server_memory_mb: u32,
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

        // Server management is opt-in: without an agent URL the panel runs
        // exactly as before and the servers page reports the runtime as
        // unavailable. An agent URL without a token is always a mistake, and a
        // dangerous one, so it is rejected rather than silently ignored.
        let agent_url = non_empty(get("AGENT_URL"));
        let agent_token = non_empty(get("AGENT_TOKEN"));
        if agent_url.is_some() && agent_token.is_none() {
            return Err(ConfigError::new(
                "AGENT_TOKEN is required when AGENT_URL is set; generate one with \
                 `openssl rand -hex 32` and use the same value for the agent",
            ));
        }

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
            agent_url,
            agent_token,
            agent_timeout: Duration::from_secs(positive_u64(
                get("AGENT_TIMEOUT_SECS"),
                DEFAULT_AGENT_TIMEOUT_SECS,
                "AGENT_TIMEOUT_SECS",
            )?),
            paper_api_url: non_empty(get("PAPER_API_URL"))
                .unwrap_or_else(|| DEFAULT_PAPER_API_URL.to_string()),
            paper_timeout: Duration::from_secs(positive_u64(
                get("PAPER_TIMEOUT_SECS"),
                DEFAULT_PAPER_TIMEOUT_SECS,
                "PAPER_TIMEOUT_SECS",
            )?),
            paper_cache_ttl: Duration::from_secs(positive_u64(
                get("PAPER_CACHE_TTL_SECS"),
                DEFAULT_PAPER_CACHE_TTL_SECS,
                "PAPER_CACHE_TTL_SECS",
            )?),
            default_server_memory_mb: match non_empty(get("DEFAULT_SERVER_MEMORY_MB")) {
                Some(raw) => raw
                    .parse::<u32>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| {
                        ConfigError::new("DEFAULT_SERVER_MEMORY_MB must be a positive integer")
                    })?,
                None => DEFAULT_SERVER_MEMORY_MB,
            },
        })
    }
}

/// Reads a positive integer setting, falling back to `default` when unset.
fn positive_u64(raw: Option<String>, default: u64, name: &str) -> Result<u64, ConfigError> {
    match non_empty(raw) {
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| ConfigError::new(format!("{name} must be a positive integer"))),
        None => Ok(default),
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
