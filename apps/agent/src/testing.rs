//! Test doubles for the container engine.
//!
//! [`StubEngine`] implements [`ContainerEngine`] without touching Docker or
//! Podman, so the agent's HTTP layer can be exercised for real (real router,
//! real handlers, real parsing) while the runtime is simulated. It is compiled
//! unconditionally because the main API's integration tests build an agent
//! router too.

use std::sync::Mutex;

use crate::command::Invocation;
use crate::engine::{BoxFuture, ContainerEngine, EngineError, RunOutput};

/// How a [`StubEngine`] replies to a command.
pub type Responder = Box<dyn Fn(&Invocation) -> Result<RunOutput, EngineError> + Send + Sync>;

/// Records every invocation and answers with canned output.
pub struct StubEngine {
    invocations: Mutex<Vec<Invocation>>,
    responder: Responder,
}

impl StubEngine {
    /// Answers every command with success and empty output.
    pub fn new() -> Self {
        Self::with_responder(|_| Ok(RunOutput::ok("")))
    }

    /// Answers every command via `responder`.
    pub fn with_responder(
        responder: impl Fn(&Invocation) -> Result<RunOutput, EngineError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            invocations: Mutex::new(Vec::new()),
            responder: Box::new(responder),
        }
    }

    /// Every invocation the engine received, in order.
    pub fn invocations(&self) -> Vec<Invocation> {
        self.invocations
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// The subcommand of every invocation, in order (e.g. `run`, `inspect`).
    pub fn subcommands(&self) -> Vec<String> {
        self.invocations()
            .iter()
            .filter_map(|invocation| invocation.args.first().cloned())
            .collect()
    }

    /// How many commands the engine ran.
    pub fn count(&self) -> usize {
        self.invocations.lock().map(|guard| guard.len()).unwrap_or(0)
    }
}

impl Default for StubEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerEngine for StubEngine {
    fn run<'a>(
        &'a self,
        invocation: &'a Invocation,
    ) -> BoxFuture<'a, Result<RunOutput, EngineError>> {
        Box::pin(async move {
            if let Ok(mut guard) = self.invocations.lock() {
                guard.push(invocation.clone());
            }
            (self.responder)(invocation)
        })
    }
}

/// Builds `inspect` output for a managed container, as the runtime would.
pub fn inspect_json(id: &str, running: bool, state: &str) -> String {
    serde_json::json!([{
        "State": { "Status": state, "Running": running },
        "Config": { "Labels": { "pumapanel.managed": "true", "pumapanel.server": id } },
    }])
    .to_string()
}
