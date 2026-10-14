//! Container-runtime detection.
//!
//! The agent has to answer two questions before it can do anything: *where am
//! I running?* (bare metal, or a container on Docker/Podman) and *which runtime
//! should run Minecraft servers?*
//!
//! The deployment policy is:
//!
//! | Where the agent runs   | Runtime used      |
//! | ---------------------- | ----------------- |
//! | Docker container       | Docker            |
//! | Podman container       | Podman            |
//! | Bare metal             | Podman (preferred), else Docker |
//!
//! `RUNTIME=docker|podman` overrides the inferred choice.
//!
//! Detection reads well-known container markers rather than inspecting the
//! runtime, so it needs no privileges: Docker writes `/.dockerenv`, Podman
//! writes `/run/.containerenv` and sets `container=podman`.

use std::path::{Path, PathBuf};

use crate::wire::{HostEnvironment, Runtime, RuntimeInfo};

/// Filesystem and environment signals used to classify the host.
///
/// Kept as plain data so [`resolve`] is a pure function and every signal can be
/// exercised in tests.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    /// `/.dockerenv` exists (created by Docker for every container).
    pub docker_marker: bool,
    /// `/run/.containerenv` exists (created by Podman).
    pub podman_marker: bool,
    /// The `container` environment variable, when set.
    pub container_env: Option<String>,
    /// Path to the `docker` CLI, if it is on `PATH`.
    pub docker_binary: Option<PathBuf>,
    /// Path to the `podman` CLI, if it is on `PATH`.
    pub podman_binary: Option<PathBuf>,
}

impl Probe {
    /// Reads the signals from the current process and host.
    pub fn current() -> Self {
        Self {
            docker_marker: Path::new("/.dockerenv").exists(),
            podman_marker: Path::new("/run/.containerenv").exists(),
            container_env: std::env::var("container").ok(),
            docker_binary: find_binary("docker"),
            podman_binary: find_binary("podman"),
        }
    }
}

/// The runtime the agent will actually use, or why it cannot use one.
#[derive(Debug, Clone)]
pub enum Resolved {
    Ready { runtime: Runtime, binary: PathBuf },
    Unavailable {
        runtime: Option<Runtime>,
        message: String,
    },
}

/// Classification of the host plus the resolved runtime.
#[derive(Debug, Clone)]
pub struct Detection {
    pub host_environment: HostEnvironment,
    pub resolved: Resolved,
}

impl Detection {
    /// The runtime that container operations should go through.
    pub fn runtime(&self) -> Option<Runtime> {
        match &self.resolved {
            Resolved::Ready { runtime, .. } => Some(*runtime),
            Resolved::Unavailable { runtime, .. } => *runtime,
        }
    }

    /// The runtime CLI to invoke, when it was found.
    pub fn binary(&self) -> Option<&Path> {
        match &self.resolved {
            Resolved::Ready { binary, .. } => Some(binary.as_path()),
            Resolved::Unavailable { .. } => None,
        }
    }

    /// The reason no runtime is usable, when that is the case.
    pub fn unavailable_message(&self) -> Option<&str> {
        match &self.resolved {
            Resolved::Ready { .. } => None,
            Resolved::Unavailable { message, .. } => Some(message.as_str()),
        }
    }

    /// Builds the wire representation, given the runtime's reported version.
    pub fn to_info(&self, version: Option<String>) -> RuntimeInfo {
        let runtime = self.runtime();
        let message = self.unavailable_message().map(str::to_string);
        RuntimeInfo {
            runtime,
            host_environment: self.host_environment,
            // Reachable only when a binary was found *and* it answered.
            available: self.binary().is_some() && version.is_some(),
            version,
            message,
        }
    }
}

/// Detects the runtime from the live host.
pub fn detect(override_runtime: Option<Runtime>) -> Detection {
    resolve(override_runtime, &Probe::current())
}

/// Pure detection over an explicit [`Probe`].
pub fn resolve(override_runtime: Option<Runtime>, probe: &Probe) -> Detection {
    let host_environment = classify(probe);
    let inferred = match host_environment {
        HostEnvironment::PodmanContainer => Some(Runtime::Podman),
        HostEnvironment::DockerContainer => Some(Runtime::Docker),
        HostEnvironment::BareMetal => None,
    };

    // An explicit override always wins; inside a container the runtime is fixed
    // by whichever engine hosts us; on bare metal prefer Podman.
    let desired = override_runtime.or(inferred).or_else(|| {
        if probe.podman_binary.is_some() {
            Some(Runtime::Podman)
        } else if probe.docker_binary.is_some() {
            Some(Runtime::Docker)
        } else {
            None
        }
    });

    let resolved = match desired {
        None => Resolved::Unavailable {
            runtime: None,
            message: "No container runtime found. Install Podman (Docker is also supported)."
                .to_string(),
        },
        Some(runtime) => {
            let binary = match runtime {
                Runtime::Docker => probe.docker_binary.clone(),
                Runtime::Podman => probe.podman_binary.clone(),
            };
            match binary {
                Some(binary) => Resolved::Ready { runtime, binary },
                None => Resolved::Unavailable {
                    runtime: Some(runtime),
                    message: format!(
                        "The `{}` CLI was not found on PATH, so {} cannot manage servers.",
                        runtime.binary_name(),
                        runtime.label()
                    ),
                },
            }
        }
    };

    Detection {
        host_environment,
        resolved,
    }
}

/// Classifies where the agent process is running from the available signals.
fn classify(probe: &Probe) -> HostEnvironment {
    let container_env = probe.container_env.as_deref().map(str::trim);
    // Podman also writes `/.dockerenv` in some configurations, so its own
    // markers are checked first.
    if probe.podman_marker || container_env == Some("podman") {
        HostEnvironment::PodmanContainer
    } else if probe.docker_marker || container_env == Some("docker") {
        HostEnvironment::DockerContainer
    } else {
        HostEnvironment::BareMetal
    }
}

/// Finds an executable on `PATH`.
fn find_binary(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe() -> Probe {
        Probe::default()
    }

    fn ready(detection: &Detection) -> (Runtime, &Path) {
        match &detection.resolved {
            Resolved::Ready { runtime, binary } => (*runtime, binary.as_path()),
            Resolved::Unavailable { message, .. } => panic!("expected a runtime, got: {message}"),
        }
    }

    #[test]
    fn bare_metal_prefers_podman() {
        let probe = Probe {
            docker_binary: Some(PathBuf::from("/usr/bin/docker")),
            podman_binary: Some(PathBuf::from("/usr/bin/podman")),
            ..probe()
        };

        let detection = resolve(None, &probe);

        assert_eq!(detection.host_environment, HostEnvironment::BareMetal);
        let (runtime, binary) = ready(&detection);
        assert_eq!(runtime, Runtime::Podman);
        assert_eq!(binary, Path::new("/usr/bin/podman"));
    }

    #[test]
    fn bare_metal_falls_back_to_docker() {
        let probe = Probe {
            docker_binary: Some(PathBuf::from("/usr/bin/docker")),
            ..probe()
        };

        let (runtime, _) = ready(&resolve(None, &probe));

        assert_eq!(runtime, Runtime::Docker);
    }

    #[test]
    fn bare_metal_without_any_runtime_is_unavailable() {
        let detection = resolve(None, &probe());

        assert_eq!(detection.host_environment, HostEnvironment::BareMetal);
        assert_eq!(detection.runtime(), None);
        assert!(detection.unavailable_message().is_some());
    }

    #[test]
    fn docker_container_uses_docker_even_when_podman_is_present() {
        let probe = Probe {
            docker_marker: true,
            docker_binary: Some(PathBuf::from("/usr/bin/docker")),
            podman_binary: Some(PathBuf::from("/usr/bin/podman")),
            ..probe()
        };

        let detection = resolve(None, &probe);

        assert_eq!(detection.host_environment, HostEnvironment::DockerContainer);
        let (runtime, _) = ready(&detection);
        assert_eq!(runtime, Runtime::Docker);
    }

    #[test]
    fn podman_marker_wins_over_docker_marker() {
        // Podman writes both markers when emulating Docker.
        let probe = Probe {
            docker_marker: true,
            podman_marker: true,
            docker_binary: Some(PathBuf::from("/usr/bin/docker")),
            podman_binary: Some(PathBuf::from("/usr/bin/podman")),
            ..probe()
        };

        let detection = resolve(None, &probe);

        assert_eq!(detection.host_environment, HostEnvironment::PodmanContainer);
        let (runtime, _) = ready(&detection);
        assert_eq!(runtime, Runtime::Podman);
    }

    #[test]
    fn container_env_is_honoured() {
        let probe = Probe {
            container_env: Some("podman".to_string()),
            podman_binary: Some(PathBuf::from("/usr/bin/podman")),
            ..probe()
        };

        let detection = resolve(None, &probe);

        assert_eq!(detection.host_environment, HostEnvironment::PodmanContainer);
    }

    #[test]
    fn override_wins_everywhere() {
        let probe = Probe {
            podman_marker: true,
            docker_binary: Some(PathBuf::from("/usr/bin/docker")),
            podman_binary: Some(PathBuf::from("/usr/bin/podman")),
            ..probe()
        };

        let detection = resolve(Some(Runtime::Docker), &probe);

        // The environment is still reported honestly...
        assert_eq!(detection.host_environment, HostEnvironment::PodmanContainer);
        // ...but the override decides the runtime.
        let (runtime, _) = ready(&detection);
        assert_eq!(runtime, Runtime::Docker);
    }

    #[test]
    fn a_container_without_its_cli_is_unavailable() {
        let probe = Probe {
            docker_marker: true,
            podman_binary: Some(PathBuf::from("/usr/bin/podman")),
            ..probe()
        };

        let detection = resolve(None, &probe);

        assert_eq!(detection.runtime(), Some(Runtime::Docker));
        assert!(detection.binary().is_none());
        assert!(detection.unavailable_message().is_some());
    }
}
