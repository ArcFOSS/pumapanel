//! Execution of container-runtime commands.
//!
//! The engine is a trait so the agent's HTTP layer can be tested without a
//! real Docker or Podman daemon: production uses [`CliEngine`], tests use the
//! recording stub in [`crate::testing`].

use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;

use tokio::process::Command;

use crate::command::{self, Invocation};

/// A boxed future, so [`ContainerEngine`] stays object-safe.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The result of running a runtime command.
///
/// A non-zero exit is *not* an error at this layer: `inspect` on a missing
/// container and `volume rm` on a missing volume both exit non-zero and both
/// mean something to the caller. Only a failure to spawn the CLI is an error.
#[derive(Debug, Clone)]
pub struct RunOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl RunOutput {
    /// A successful run with the given standard output.
    pub fn ok(stdout: impl Into<String>) -> Self {
        Self {
            success: true,
            code: Some(0),
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// A failed run with the given standard error.
    pub fn failed(stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            code: Some(1),
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    /// The most useful detail for an error message.
    pub fn detail(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            self.stderr.clone()
        }
    }

    /// Whether the runtime reported that the target does not exist.
    ///
    /// Docker says `No such container`, Podman `no such volume`: matching on
    /// the words lets removal stay idempotent on both engines.
    pub fn is_not_found(&self) -> bool {
        let text = format!("{} {}", self.stdout, self.stderr).to_ascii_lowercase();
        text.contains("no such") || text.contains("not found") || text.contains("does not exist")
    }
}

/// Failure to run a runtime command at all (the CLI is missing or broken).
#[derive(Debug, Clone)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for EngineError {}

/// Runs container-runtime commands.
pub trait ContainerEngine: Send + Sync + 'static {
    fn run<'a>(
        &'a self,
        invocation: &'a Invocation,
    ) -> BoxFuture<'a, Result<RunOutput, EngineError>>;
}

/// The real engine: spawns the CLI and captures its output.
#[derive(Debug, Default, Clone, Copy)]
pub struct CliEngine;

impl ContainerEngine for CliEngine {
    fn run<'a>(
        &'a self,
        invocation: &'a Invocation,
    ) -> BoxFuture<'a, Result<RunOutput, EngineError>> {
        Box::pin(async move {
            let output = Command::new(&invocation.program)
                .args(&invocation.args)
                .stdin(Stdio::null())
                .output()
                .await
                .map_err(|error| {
                    EngineError(format!(
                        "could not run `{}`: {error}",
                        invocation.program.display()
                    ))
                })?;

            Ok(RunOutput {
                success: output.status.success(),
                code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            })
        })
    }
}

/// The fields the agent reads back from one container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedContainer {
    /// PumaPanel server id, from the `pumapanel.server` label.
    pub id: String,
    pub running: bool,
    /// The runtime's raw status string.
    pub state: String,
}

/// Reads a managed container out of `inspect` output.
///
/// Both engines print a JSON array; the first element carries `State` and
/// `Config.Labels`. Returns `None` for output that is not a managed container,
/// which is what an inspect of a foreign or missing container looks like.
pub fn parse_inspect(json: &str) -> Option<ParsedContainer> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let first = value.as_array()?.first()?;

    let id = first
        .get("Config")?
        .get("Labels")?
        .get(command::SERVER_LABEL)?
        .as_str()?
        .to_string();

    let state = first.get("State")?;
    Some(ParsedContainer {
        id,
        running: state.get("Running").and_then(serde_json::Value::as_bool).unwrap_or(false),
        state: state
            .get("Status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from a real `podman inspect` of a managed container, whose
    /// `.[0].State` object is what the smoke test printed.
    const RUNNING: &str = r#"[{"Id":"41c2b00e","State":{"OciVersion":"1.3.0","Status":"running",
        "Running":true,"Paused":false,"ExitCode":0},
        "Config":{"Labels":{"pumapanel.managed":"true","pumapanel.server":"smoke"}}}]"#;

    #[test]
    fn parses_a_running_container() {
        assert_eq!(
            parse_inspect(RUNNING),
            Some(ParsedContainer {
                id: "smoke".to_string(),
                running: true,
                state: "running".to_string(),
            })
        );
    }

    #[test]
    fn parses_a_stopped_container() {
        let json = r#"[{"State":{"Status":"exited","Running":false},
            "Config":{"Labels":{"pumapanel.server":"smoke"}}}]"#;

        let parsed = parse_inspect(json).expect("parsed");

        assert!(!parsed.running);
        assert_eq!(parsed.state, "exited");
    }

    #[test]
    fn ignores_containers_the_agent_does_not_manage() {
        let json = r#"[{"State":{"Status":"running","Running":true},"Config":{"Labels":{}}}]"#;

        assert_eq!(parse_inspect(json), None);
    }

    #[test]
    fn survives_unexpected_output() {
        assert_eq!(parse_inspect(""), None);
        assert_eq!(parse_inspect("not json"), None);
        assert_eq!(parse_inspect("[]"), None);
    }

    #[test]
    fn recognises_missing_targets() {
        assert!(RunOutput::failed("Error: No such container: pumapanel-abc").is_not_found());
        assert!(RunOutput::failed("Error: no such volume pumapanel-abc").is_not_found());
        assert!(!RunOutput::failed("Error: port is already allocated").is_not_found());
    }
}
