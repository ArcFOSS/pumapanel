//! High-level server operations driven through a [`ContainerEngine`].
//!
//! Every method maps one agent-facing concept (`create`, `start`, ...) onto
//! runtime commands from [`crate::command`], and turns their output into the
//! agent's [`ServerState`].

use std::path::Path;
use std::sync::Arc;

use crate::command::{self, CreateSpec, Invocation, container_name, is_valid_id};
use crate::config::Config;
use crate::detect::Detection;
use crate::engine::{ContainerEngine, EngineError, RunOutput, parse_inspect};
use crate::error::AgentError;
use crate::memory::{HostMemory, ProcMeminfo};
use crate::wire::{CreateServerRequest, RuntimeInfo, ServerList, ServerState};

/// Accepted JVM heap range, in MiB.
///
/// Public so the API can validate a request against the same bounds the agent
/// enforces instead of keeping a second copy that could drift.
pub const MIN_MEMORY_MB: u32 = 256;
pub const MAX_MEMORY_MB: u32 = 65_536;

/// Container management for the agent's HTTP layer.
pub struct Operations {
    engine: Arc<dyn ContainerEngine>,
    detection: Detection,
    config: Config,
    memory: Arc<dyn HostMemory>,
}

impl Operations {
    /// Builds operations that size `all_memory` servers from the host's own
    /// `/proc/meminfo`.
    pub fn new(engine: Arc<dyn ContainerEngine>, detection: Detection, config: Config) -> Self {
        Self::with_memory(engine, detection, config, Arc::new(ProcMeminfo))
    }

    /// Builds operations with an explicit memory source, so tests can pin the
    /// free memory an `all_memory` server would see.
    pub fn with_memory(
        engine: Arc<dyn ContainerEngine>,
        detection: Detection,
        config: Config,
        memory: Arc<dyn HostMemory>,
    ) -> Self {
        Self {
            engine,
            detection,
            config,
            memory,
        }
    }

    pub fn detection(&self) -> &Detection {
        &self.detection
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Reports which runtime was detected and whether it answered.
    pub async fn info(&self) -> RuntimeInfo {
        let Some(binary) = self.detection.binary() else {
            return self.detection.to_info(None);
        };

        match self.engine.run(&command::version(binary)).await {
            Ok(output) if output.success => self.detection.to_info(Some(output.stdout.clone())),
            _ => {
                let mut info = self.detection.to_info(None);
                if info.message.is_none() {
                    info.message =
                        Some("The container runtime did not answer `--version`.".to_string());
                }
                info
            }
        }
    }

    /// Every container the agent manages, with its live state.
    ///
    /// Names come from one `ps` call filtered on the managed label; each
    /// container is then inspected, because `inspect` is the only output that
    /// is identical across Docker and Podman.
    pub async fn list(&self) -> Result<ServerList, AgentError> {
        let binary = self.binary()?;
        let listed = self.run_ok(&command::list_managed(binary)).await?;

        let mut servers = Vec::new();
        for name in listed
            .stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            if let Some(state) = self.inspect_container(name).await? {
                servers.push(state);
            }
        }
        servers.sort_by(|left, right| left.id.cmp(&right.id));

        Ok(ServerList { servers })
    }

    /// Creates and starts a PaperMC server container.
    pub async fn create(&self, request: &CreateServerRequest) -> Result<ServerState, AgentError> {
        self.validate(request)?;

        let id = request.id.trim();
        let container = container_name(id);
        let image = command::image_for_java(&self.config.image, request.java_min);
        let spec = CreateSpec {
            id,
            version: request.version.trim(),
            memory_mb: self.memory_for(request)?,
            host_port: request.host_port,
            image: &image,
        };

        let binary = self.binary()?;
        self.run_ok(&command::create(binary, &spec)).await?;

        self.inspect_container(&container)
            .await?
            .ok_or_else(|| AgentError::internal("the container vanished right after creation"))
    }

    /// Starts a stopped server.
    pub async fn start(&self, id: &str) -> Result<ServerState, AgentError> {
        let container = self.container_for(id)?;
        let binary = self.binary()?;
        self.run_ok(&command::start(binary, id.trim())).await?;
        self.require(&container).await
    }

    /// Stops a running server, giving it time to save its world.
    pub async fn stop(&self, id: &str) -> Result<ServerState, AgentError> {
        let container = self.container_for(id)?;
        let binary = self.binary()?;
        self.run_ok(&command::stop(binary, id.trim())).await?;
        self.require(&container).await
    }

    /// Removes a server, optionally deleting its world data.
    ///
    /// Removal is idempotent: a container or volume that is already gone is
    /// treated as success, so a failed delete can simply be retried.
    pub async fn remove(&self, id: &str, remove_data: bool) -> Result<(), AgentError> {
        let id = id.trim();
        self.container_for(id)?;
        let binary = self.binary()?;

        let removed = self.run(&command::remove(binary, id)).await?;
        if !removed.success && !removed.is_not_found() {
            return Err(AgentError::Runtime(format!(
                "`{} remove` failed: {}",
                binary.display(),
                removed.detail()
            )));
        }

        if remove_data {
            let volume = self.run(&command::remove_volume(binary, id)).await?;
            if !volume.success && !volume.is_not_found() {
                return Err(AgentError::Runtime(format!(
                    "`{} volume rm` failed: {}",
                    binary.display(),
                    volume.detail()
                )));
            }
        }

        Ok(())
    }

    /// Rejects requests the runtime should never see.
    fn validate(&self, request: &CreateServerRequest) -> Result<(), AgentError> {
        if !is_valid_id(request.id.trim()) {
            return Err(AgentError::BadRequest(
                "`id` must be a short, name-safe identifier".to_string(),
            ));
        }
        if request.version.trim().is_empty() {
            return Err(AgentError::BadRequest("`version` is required".to_string()));
        }
        if request.host_port == 0 {
            return Err(AgentError::BadRequest(
                "`hostPort` must be between 1 and 65535".to_string(),
            ));
        }

        if !request.all_memory {
            let memory = effective_memory(request.memory_mb, &self.config);
            if !(MIN_MEMORY_MB..=MAX_MEMORY_MB).contains(&memory) {
                return Err(AgentError::BadRequest(format!(
                    "`memoryMb` must be between {MIN_MEMORY_MB} and {MAX_MEMORY_MB}"
                )));
            }
        }

        Ok(())
    }

    /// The JVM heap a request should run with.
    ///
    /// An `all_memory` server gets whatever the host has free right now. The
    /// number is resolved once, here, because a container's environment is fixed
    /// when it is created.
    fn memory_for(&self, request: &CreateServerRequest) -> Result<u32, AgentError> {
        if !request.all_memory {
            return Ok(effective_memory(request.memory_mb, &self.config));
        }

        self.memory
            .available_mb()
            .filter(|available| *available > 0)
            .ok_or_else(|| {
                AgentError::Unavailable(
                    "Could not determine the host's free memory for this server.".to_string(),
                )
            })
    }

    /// The runtime CLI, or a 503 explaining why there is none.
    fn binary(&self) -> Result<&Path, AgentError> {
        self.detection.binary().ok_or_else(|| {
            AgentError::Unavailable(
                self.detection
                    .unavailable_message()
                    .unwrap_or("No container runtime is available.")
                    .to_string(),
            )
        })
    }

    /// Validates a server id and returns its container name.
    fn container_for(&self, id: &str) -> Result<String, AgentError> {
        if !is_valid_id(id.trim()) {
            return Err(AgentError::BadRequest(
                "`id` must be a short, name-safe identifier".to_string(),
            ));
        }
        Ok(container_name(id.trim()))
    }

    /// Runs a command, mapping a non-zero exit into a runtime error.
    async fn run_ok(&self, invocation: &Invocation) -> Result<RunOutput, AgentError> {
        let output = self.run(invocation).await?;
        if output.success {
            return Ok(output);
        }

        Err(AgentError::Runtime(format!(
            "`{}` failed: {}",
            invocation.program.display(),
            output.detail()
        )))
    }

    /// Runs a command, treating only a spawn failure as an error.
    async fn run(&self, invocation: &Invocation) -> Result<RunOutput, AgentError> {
        self.engine
            .run(invocation)
            .await
            .map_err(|EngineError(message)| AgentError::Unavailable(message))
    }

    /// Inspects a container by name, `None` when it does not exist.
    async fn inspect_container(&self, container: &str) -> Result<Option<ServerState>, AgentError> {
        let binary = self.binary()?;
        let output = self.run(&command::inspect(binary, container)).await?;
        if !output.success {
            return Ok(None);
        }

        let parsed = parse_inspect(&output.stdout).ok_or_else(|| {
            AgentError::internal(format!("could not parse `inspect` output for {container}"))
        })?;

        Ok(Some(ServerState {
            id: parsed.id,
            container: container.to_string(),
            state: parsed.state,
            running: parsed.running,
        }))
    }

    /// Inspects a container, failing with 404 when it is gone.
    async fn require(&self, container: &str) -> Result<ServerState, AgentError> {
        self.inspect_container(container)
            .await?
            .ok_or(AgentError::NotFound)
    }
}

/// Falls back to the configured default when a request omits a heap size.
fn effective_memory(requested: u32, config: &Config) -> u32 {
    if requested == 0 {
        config.default_memory_mb
    } else {
        requested
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::detect::{Detection, Resolved};
    use crate::testing::{StubEngine, inspect_json};
    use crate::wire::{HostEnvironment, Runtime};

    const ID: &str = "6f9619ff-816d-4d01-b42d-00cf4fc964ff";

    fn detection(resolved: Resolved) -> Detection {
        Detection {
            host_environment: HostEnvironment::BareMetal,
            resolved,
        }
    }

    fn ready() -> Detection {
        detection(Resolved::Ready {
            runtime: Runtime::Podman,
            binary: PathBuf::from("/usr/bin/podman"),
        })
    }

    fn config() -> Config {
        Config {
            bind: "127.0.0.1:9091".to_string(),
            token: "0123456789abcdef0123456789abcdef".to_string(),
            runtime_override: None,
            image: "itzg/minecraft-server".to_string(),
            default_memory_mb: 2048,
        }
    }

    fn request() -> CreateServerRequest {
        CreateServerRequest {
            id: ID.to_string(),
            version: "1.21.8".to_string(),
            memory_mb: 4096,
            host_port: 25566,
            java_min: Some(21),
        }
    }

    #[tokio::test]
    async fn create_runs_the_container_then_reads_its_state() {
        let engine = Arc::new(StubEngine::with_responder(|invocation| {
            if invocation.args.first().map(String::as_str) == Some("inspect") {
                return Ok(RunOutput::ok(inspect_json(ID, true, "running")));
            }
            Ok(RunOutput::ok(""))
        }));
        let operations = Operations::new(engine.clone(), ready(), config());

        let state = operations.create(&request()).await.expect("created");

        assert_eq!(state.id, ID);
        assert!(state.running);
        assert_eq!(engine.subcommands(), vec!["run", "inspect"]);
        let create = &engine.invocations()[0];
        assert!(create.args.iter().any(|arg| arg == "itzg/minecraft-server:java21"));
    }

    #[tokio::test]
    async fn create_is_rejected_before_it_reaches_the_runtime() {
        let engine = Arc::new(StubEngine::new());
        let operations = Operations::new(engine.clone(), ready(), config());

        let mut bad = request();
        bad.id = "not safe".to_string();
        assert!(matches!(
            operations.create(&bad).await,
            Err(AgentError::BadRequest(_))
        ));

        let mut tiny = request();
        tiny.memory_mb = 1;
        assert!(matches!(
            operations.create(&tiny).await,
            Err(AgentError::BadRequest(_))
        ));

        assert_eq!(engine.count(), 0);
    }

    #[tokio::test]
    async fn create_reports_runtime_failures() {
        let engine = Arc::new(StubEngine::with_responder(|_| {
            Ok(RunOutput::failed("Error: port is already allocated"))
        }));
        let operations = Operations::new(engine, ready(), config());

        let error = operations.create(&request()).await.expect_err("failed");

        assert!(matches!(error, AgentError::Runtime(_)));
    }

    #[tokio::test]
    async fn without_a_runtime_everything_is_unavailable() {
        let engine = Arc::new(StubEngine::new());
        let unavailable = detection(Resolved::Unavailable {
            runtime: None,
            message: "No container runtime found.".to_string(),
        });
        let operations = Operations::new(engine.clone(), unavailable, config());

        assert!(matches!(
            operations.list().await,
            Err(AgentError::Unavailable(_))
        ));
        assert!(matches!(
            operations.create(&request()).await,
            Err(AgentError::Unavailable(_))
        ));
        assert_eq!(engine.count(), 0);
    }

    #[tokio::test]
    async fn removing_a_missing_container_is_idempotent() {
        let engine = Arc::new(StubEngine::with_responder(|_| {
            Ok(RunOutput::failed("Error: No such container: pumapanel-x"))
        }));
        let operations = Operations::new(engine.clone(), ready(), config());

        operations.remove(ID, true).await.expect("idempotent");

        let invocations = engine.invocations();
        assert_eq!(invocations[0].args, vec!["rm", "--force", &container_name(ID)]);
        assert_eq!(invocations[1].args, vec!["volume", "rm", &container_name(ID)]);
    }

    #[tokio::test]
    async fn info_reports_the_runtime_version() {
        let engine = Arc::new(StubEngine::with_responder(|_| {
            Ok(RunOutput::ok("podman version 5.6.1"))
        }));
        let operations = Operations::new(engine, ready(), config());

        let info = operations.info().await;

        assert_eq!(info.runtime, Some(Runtime::Podman));
        assert!(info.available);
        assert_eq!(info.version.as_deref(), Some("podman version 5.6.1"));
    }

    #[tokio::test]
    async fn start_reports_missing_containers_as_not_found() {
        let engine = Arc::new(StubEngine::with_responder(|invocation| {
            if invocation.args.first().map(String::as_str) == Some("inspect") {
                return Ok(RunOutput::failed("Error: No such container"));
            }
            Ok(RunOutput::ok(""))
        }));
        let operations = Operations::new(engine, ready(), config());

        assert!(matches!(operations.start(ID).await, Err(AgentError::NotFound)));
    }
}
