use crate::wire::Runtime;

/// Default bind address. Loopback-only, because the API normally shares the
/// host with the agent; containerized deployments override it to `0.0.0.0`.
pub const DEFAULT_BIND: &str = "127.0.0.1:9091";
/// Default Minecraft server image. `itzg/minecraft-server` drives PaperMC
/// (`TYPE=PAPER`) and publishes `java8`/`java17`/`java21`/`java25` tags.
pub const DEFAULT_IMAGE: &str = "itzg/minecraft-server";
/// Default JVM heap (MiB) for new servers.
pub const DEFAULT_MEMORY_MB: u32 = 2048;
/// Shortest accepted `AGENT_TOKEN`, so a weak shared secret is rejected.
const MIN_TOKEN_LEN: usize = 16;

/// Invalid or missing agent configuration.
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

/// Runtime configuration for the agent, resolved from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP API binds to.
    pub bind: String,
    /// Shared secret required on every request (`Authorization: Bearer`).
    pub token: String,
    /// Forces a runtime instead of inferring it (`RUNTIME=docker|podman`.
    pub runtime_override: Option<Runtime>,
    /// Container image that runs PaperMC.
    pub image: String,
    /// Heap size used when a request omits one.
    pub default_memory_mb: u32,
}

impl Config {
    /// Loads configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::load(|key| std::env::var(key).ok())
    }

    /// Loads configuration from an arbitrary source, so tests can exercise
    /// validation without touching process-wide environment state.
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let token = non_empty(get("AGENT_TOKEN")).ok_or_else(|| {
            ConfigError::new(
                "AGENT_TOKEN is required; generate one with `openssl rand -hex 32` \
                 and use the same value for the API",
            )
        })?;
        if token.len() < MIN_TOKEN_LEN {
            return Err(ConfigError::new(format!(
                "AGENT_TOKEN must be at least {MIN_TOKEN_LEN} characters"
            )));
        }

        let default_memory_mb = match non_empty(get("DEFAULT_MEMORY_MB")) {
            Some(raw) => raw
                .parse::<u32>()
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| {
                    ConfigError::new("DEFAULT_MEMORY_MB must be a positive integer")
                })?,
            None => DEFAULT_MEMORY_MB,
        };

        Ok(Self {
            bind: non_empty(get("AGENT_BIND")).unwrap_or_else(|| DEFAULT_BIND.to_string()),
            token,
            runtime_override: parse_runtime(get("RUNTIME"))?,
            image: non_empty(get("MINECRAFT_IMAGE"))
                .unwrap_or_else(|| DEFAULT_IMAGE.to_string()),
            default_memory_mb,
        })
    }
}

/// Parses `RUNTIME`: `auto` (the default) infers the runtime, while `docker`
/// and `podman` force one.
fn parse_runtime(raw: Option<String>) -> Result<Option<Runtime>, ConfigError> {
    match raw.as_deref().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        None | Some("" | "auto") => Ok(None),
        Some("docker") => Ok(Some(Runtime::Docker)),
        Some("podman") => Ok(Some(Runtime::Podman)),
        Some(other) => Err(ConfigError::new(format!(
            "RUNTIME must be `auto`, `docker`, or `podman` (got `{other}`)"
        ))),
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        Config::load(|key| {
            pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_string())
        })
    }

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn requires_a_token() {
        assert!(load(&[]).is_err());
        assert!(load(&[("AGENT_TOKEN", "short")]).is_err());
    }

    #[test]
    fn applies_defaults() {
        let config = load(&[("AGENT_TOKEN", TOKEN)]).expect("valid config");
        assert_eq!(config.bind, DEFAULT_BIND);
        assert_eq!(config.image, DEFAULT_IMAGE);
        assert_eq!(config.default_memory_mb, DEFAULT_MEMORY_MB);
        assert_eq!(config.runtime_override, None);
    }

    #[test]
    fn parses_runtime_override() {
        assert_eq!(
            load(&[("AGENT_TOKEN", TOKEN), ("RUNTIME", "docker")])
                .expect("valid config")
                .runtime_override,
            Some(Runtime::Docker)
        );
        assert_eq!(
            load(&[("AGENT_TOKEN", TOKEN), ("RUNTIME", "auto")])
                .expect("valid config")
                .runtime_override,
            None
        );
        assert!(load(&[("AGENT_TOKEN", TOKEN), ("RUNTIME", "lxc")]).is_err());
    }

    #[test]
    fn rejects_a_non_numeric_memory_default() {
        assert!(load(&[("AGENT_TOKEN", TOKEN), ("DEFAULT_MEMORY_MB", "lots")]).is_err());
    }
}
