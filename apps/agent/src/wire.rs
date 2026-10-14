//! Wire contract between the agent and the main API.
//!
//! These types are the single source of truth for the agent's HTTP API: the
//! main API depends on this crate for the request/response shapes, so the two
//! sides cannot drift apart without failing to compile. Everything is
//! `camelCase` on the wire to match the rest of the project's JSON.

use serde::{Deserialize, Serialize};

/// The container runtime the agent drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    Docker,
    Podman,
}

impl Runtime {
    /// The CLI binary that talks to this runtime.
    pub fn binary_name(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }

    /// Human-readable name, used in messages and the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::Podman => "Podman",
        }
    }
}

/// Where the agent process itself is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostEnvironment {
    /// Installed directly on the host; container management falls back to
    /// Podman per the deployment policy.
    BareMetal,
    /// Running in a container on a Docker host.
    DockerContainer,
    /// Running in a container on a Podman host.
    PodmanContainer,
}

impl HostEnvironment {
    /// Whether the agent is running inside a container.
    pub fn is_container(self) -> bool {
        !matches!(self, Self::BareMetal)
    }
}

/// Runtime detection result, reported to the API so the UI can explain itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    /// The runtime the agent will use, or `None` when none could be resolved.
    pub runtime: Option<Runtime>,
    /// Whether the agent itself runs on bare metal or inside a container.
    pub host_environment: HostEnvironment,
    /// Whether the runtime was reachable (`<runtime> --version` succeeded).
    pub available: bool,
    /// The runtime's own version string, when it could be read.
    pub version: Option<String>,
    /// Why the runtime is unavailable, when it is.
    pub message: Option<String>,
}

/// Request body for `POST /servers`: create and start a PaperMC server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServerRequest {
    /// PumaPanel's server id. Drives the container and volume names.
    pub id: String,
    /// The Minecraft/Paper version to install, e.g. `1.21.8`.
    pub version: String,
    /// JVM heap size in MiB. Ignored when `all_memory` is set.
    pub memory_mb: u32,
    /// Size the JVM heap from all of the host's free memory instead of
    /// `memory_mb`. Resolved once, when the container is created.
    #[serde(default)]
    pub all_memory: bool,
    /// Host port published to the container's `25565`.
    pub host_port: u16,
    /// Minimum Java version PaperMC reports for `version`, used to pick the
    /// image tag. `None` falls back to the image's `latest` tag.
    pub java_min: Option<u8>,
}

/// Live state of one managed container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerState {
    /// PumaPanel's server id (from the `pumapanel.server` label).
    pub id: String,
    /// The container name.
    pub container: String,
    /// The runtime's raw status string, e.g. `running` or `exited`.
    pub state: String,
    /// Whether the container is currently running.
    pub running: bool,
}

/// Response body for `GET /servers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerList {
    pub servers: Vec<ServerState>,
}

/// Error body, matching the main API's `{ "error": { "code", "message" } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}
