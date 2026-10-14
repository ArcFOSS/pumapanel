//! Minecraft server management.
//!
//! PumaPanel owns the *record* of a server (name, version, memory, port); the
//! agent owns the *container*. Handlers here join the two: they read the panel's
//! rows from SQLite, ask the agent what is actually running, and present a single
//! view to the frontend.
//!
//! Routes are gated by [`AdminUser`], like the rest of the admin API, so only
//! admins can create, start, stop or delete a server.
//!
//! Server management is optional: a deployment without an agent still serves
//! every other page. In that case the list reports the runtime as unavailable
//! and mutations return `503` rather than failing at startup.

mod store;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use agent::operations::{MAX_MEMORY_MB, MIN_MEMORY_MB};
use agent::wire::{CreateServerRequest as AgentCreateRequest, HostEnvironment, Runtime, RuntimeInfo, ServerState};

use crate::auth::extractor::AdminUser;
use crate::error::{ApiError, FieldError};
use crate::state::AppState;

use self::store::{NewServer, ServerRecord};

/// Longest accepted display name.
const MAX_NAME_LEN: usize = 48;
/// Ports below this are reserved for well-known services, so they are refused
/// even when chosen explicitly.
const MIN_HOST_PORT: u16 = 1024;
/// Where automatic port allocation starts: Minecraft's own port.
const FIRST_HOST_PORT: u16 = agent::command::SERVER_PORT;

/// Server routes, mounted by the application router next to `auth` and `admin`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/servers", get(list_servers).post(create_server))
        .route("/admin/servers/versions", get(list_versions))
        .route("/admin/servers/{id}", delete(delete_server))
        .route("/admin/servers/{id}/start", post(start_server))
        .route("/admin/servers/{id}/stop", post(stop_server))
}

/// What the host currently reports about a server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    /// A container exists and is running.
    Running,
    /// A container exists but is stopped.
    Stopped,
    /// The panel has a record, but the host has no container for it — the
    /// container was removed outside the panel, for example.
    Missing,
    /// The host could not be asked: no agent is configured, or it is unreachable.
    Unknown,
}

/// A server as the frontend sees it: the panel's record plus the live state.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub version: String,
    pub memory_mb: u32,
    pub host_port: u16,
    pub java_min: Option<u8>,
    pub status: ServerStatus,
    pub created_at: String,
}

impl Server {
    fn view(record: ServerRecord, status: ServerStatus) -> Self {
        Self {
            id: record.id,
            name: record.name,
            version: record.version,
            memory_mb: record.memory_mb,
            host_port: record.host_port,
            java_min: record.java_min,
            status,
            created_at: record.created_at,
        }
    }
}

/// Response for a single server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerResponse {
    server: Server,
}

/// Response for the servers list: the servers, plus why they are (or are not)
/// runnable, so the page can explain itself in one round trip.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServersResponse {
    servers: Vec<Server>,
    runtime: RuntimeView,
}

/// What the panel can say about the container runtime.
///
/// Wraps the agent's own report so the *panel* can also describe the two cases
/// the agent cannot: no agent configured at all, and an agent that could not be
/// reached.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeView {
    /// `docker`, `podman`, or `null` when none could be resolved.
    pub runtime: Option<Runtime>,
    /// Whether the runtime answered.
    pub available: bool,
    /// The runtime's own version string, when it could be read.
    pub version: Option<String>,
    /// Why servers cannot run, when they cannot.
    pub message: Option<String>,
    /// Whether the agent itself runs on bare metal or inside a container.
    pub host_environment: Option<HostEnvironment>,
}

impl RuntimeView {
    /// Server management is not configured on this deployment.
    fn unconfigured() -> Self {
        Self {
            runtime: None,
            available: false,
            version: None,
            message: Some(
                "Server management is disabled: no agent is configured for this panel."
                    .to_string(),
            ),
            host_environment: None,
        }
    }

    /// The agent is configured but could not be reached.
    fn unreachable(error: ApiError) -> Self {
        Self {
            runtime: None,
            available: false,
            version: None,
            message: Some(error.reason()),
            host_environment: None,
        }
    }

    fn from_agent(info: RuntimeInfo) -> Self {
        Self {
            runtime: info.runtime,
            available: info.available,
            version: info.version,
            message: info.message,
            host_environment: Some(info.host_environment),
        }
    }
}

/// `GET /admin/servers` — the panel's servers with their live state.
async fn list_servers(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<ServersResponse>, ApiError> {
    let records = store::list(&state.db).await?;
    let (runtime, containers) = runtime_and_containers(&state).await;

    let servers = records
        .into_iter()
        .map(|record| {
            let status = match &containers {
                // The host could not be asked, so nothing is known.
                None => ServerStatus::Unknown,
                Some(containers) => match containers.get(&record.id) {
                    Some(container) if container.running => ServerStatus::Running,
                    Some(_) => ServerStatus::Stopped,
                    None => ServerStatus::Missing,
                },
            };

            Server::view(record, status)
        })
        .collect();

    Ok(Json(ServersResponse { servers, runtime }))
}

/// Request body for `POST /admin/servers`.
///
/// The panel supplies what the caller leaves out: the id, the host port, and —
/// from the version catalogue — the minimum Java that decides the image tag.
/// `name` and `version` are optional purely so a missing one produces the panel's
/// own field error instead of an extractor rejection.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateServerRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    memory_mb: Option<u32>,
    #[serde(default)]
    host_port: Option<u16>,
}

/// `POST /admin/servers` — create and start a server.
async fn create_server(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(request): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerResponse>), ApiError> {
    // Fail before doing any work if the panel cannot run servers at all.
    state.agent.as_ref().ok_or_else(ApiError::agent_unconfigured)?;

    let name = request.name.as_deref().unwrap_or_default().trim();
    let version = request.version.as_deref().unwrap_or_default().trim();
    let memory_mb = request
        .memory_mb
        .unwrap_or(state.config.default_server_memory_mb);

    let mut fields = Vec::new();
    if name.is_empty() {
        fields.push(FieldError {
            field: "name".to_string(),
            message: "A name is required".to_string(),
        });
    } else if name.chars().count() > MAX_NAME_LEN {
        fields.push(FieldError {
            field: "name".to_string(),
            message: format!("Keep the name within {MAX_NAME_LEN} characters"),
        });
    }
    if version.is_empty() {
        fields.push(FieldError {
            field: "version".to_string(),
            message: "Choose a PaperMC version".to_string(),
        });
    }
    if !(MIN_MEMORY_MB..=MAX_MEMORY_MB).contains(&memory_mb) {
        fields.push(FieldError {
            field: "memoryMb".to_string(),
            message: format!("Memory must be between {MIN_MEMORY_MB} and {MAX_MEMORY_MB} MiB"),
        });
    }
    if let Some(port) = request.host_port
        && port < MIN_HOST_PORT
    {
        fields.push(FieldError {
            field: "hostPort".to_string(),
            message: format!("Choose a port of {MIN_HOST_PORT} or above"),
        });
    }
    if !fields.is_empty() {
        return Err(ApiError::Validation(fields));
    }

    // The version must exist upstream, and the minimum Java PaperMC reports for
    // it decides which image tag the agent runs (`java21`, `java17`, ...).
    let paper = state.paper.find(version).await?;

    let host_port = resolve_port(&state, request.host_port).await?;

    let record = store::insert(
        &state.db,
        &NewServer {
            name,
            version: &paper.version,
            memory_mb,
            host_port,
            java_min: paper.java_min,
        },
    )
    .await
    .map_err(ApiError::from_server_conflict)?;

    let agent = state
        .agent
        .as_ref()
        .ok_or_else(ApiError::agent_unconfigured)?;

    let created = agent
        .create(&AgentCreateRequest {
            id: record.id.clone(),
            version: record.version.clone(),
            memory_mb: record.memory_mb,
            host_port: record.host_port,
            java_min: record.java_min,
        })
        .await;

    let created = match created {
        Ok(container) => container,
        Err(error) => {
            // The container was never created, so the panel must not keep a
            // record of it: otherwise the list would show a server that does
            // not exist and cannot be started.
            store::delete(&state.db, &record.id).await?;
            return Err(error);
        }
    };

    Ok((
        StatusCode::CREATED,
        Json(ServerResponse {
            server: Server::view(record, status_of(&created)),
        }),
    ))
}

/// `GET /admin/servers/versions` — the PaperMC versions offered on creation.
async fn list_versions(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<crate::paper::VersionsResponse>, ApiError> {
    Ok(Json(state.paper.versions().await?))
}

/// `POST /admin/servers/{id}/start`
async fn start_server(
    State(state): State<AppState>,
    Path(id): Path<String>,
    _admin: AdminUser,
) -> Result<Json<ServerResponse>, ApiError> {
    let agent = state
        .agent
        .as_ref()
        .ok_or_else(ApiError::agent_unconfigured)?;
    let record = store::find(&state.db, &id).await?.ok_or(ApiError::NotFound)?;

    let container = agent.start(&id).await?;

    Ok(Json(ServerResponse {
        server: Server::view(record, status_of(&container)),
    }))
}

/// `POST /admin/servers/{id}/stop`
async fn stop_server(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let agent = state
        .agent
        .as_ref()
        .ok_or_else(ApiError::agent_unconfigured)?;
    let record = store::find(&state.db, &id).await?.ok_or(ApiError::NotFound)?;

    let container = agent.stop(&id).await?;

    Ok(Json(ServerResponse {
        server: Server::view(record, status_of(&container)),
    }))
}

/// Query for `DELETE /admin/servers/{id}`.
#[derive(Debug, Deserialize)]
struct DeleteServerQuery {
    /// Remove the server's world data too. Defaults to `false`, so a delete
    /// without the parameter can never destroy a world.
    #[serde(default, rename = "removeData")]
    remove_data: bool,
}

/// `DELETE /admin/servers/{id}` — remove a server, optionally with its data.
async fn delete_server(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteServerQuery>,
    _admin: AdminUser,
) -> Result<StatusCode, ApiError> {
    let agent = state
        .agent
        .as_ref()
        .ok_or_else(ApiError::agent_unconfigured)?;

    // Refuse to delete a server the panel does not know about, rather than
    // silently reporting success for a made-up id.
    store::find(&state.db, &id).await?.ok_or(ApiError::NotFound)?;

    agent.remove(&id, query.remove_data).await?;
    store::delete(&state.db, &id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Asks the agent for its runtime and its containers, concurrently.
///
/// The container map is `None` when the agent could not be asked at all, which
/// is deliberately different from "the agent has no containers": the first
/// means every status is unknown, the second means the records are orphaned.
async fn runtime_and_containers(
    state: &AppState,
) -> (RuntimeView, Option<std::collections::HashMap<String, ServerState>>) {
    let Some(agent) = state.agent.as_ref() else {
        return (RuntimeView::unconfigured(), None);
    };

    let (runtime, servers) = tokio::join!(agent.runtime(), agent.list());

    let runtime = match runtime {
        Ok(info) => RuntimeView::from_agent(info),
        Err(error) => RuntimeView::unreachable(error),
    };

    let containers = match servers {
        Ok(list) => Some(
            list.servers
                .into_iter()
                .map(|container| (container.id.clone(), container))
                .collect(),
        ),
        Err(_) => None,
    };

    (runtime, containers)
}

/// Picks the host port for a new server: the requested one when it is free, and
/// otherwise the lowest free port at or above Minecraft's own.
async fn resolve_port(state: &AppState, requested: Option<u16>) -> Result<u16, ApiError> {
    let used = store::used_ports(&state.db).await?;

    if let Some(port) = requested {
        if used.contains(&port) {
            return Err(ApiError::Validation(vec![FieldError {
                field: "hostPort".to_string(),
                message: format!("Port {port} is already used by another server"),
            }]));
        }
        return Ok(port);
    }

    first_free_port(&used).ok_or_else(|| {
        ApiError::Validation(vec![FieldError {
            field: "hostPort".to_string(),
            message: "No free host port is available".to_string(),
        }])
    })
}

/// The lowest port at or above [`FIRST_HOST_PORT`] that is not already taken.
fn first_free_port(used: &[u16]) -> Option<u16> {
    (FIRST_HOST_PORT..=u16::MAX).find(|port| !used.contains(port))
}

fn status_of(container: &ServerState) -> ServerStatus {
    if container.running {
        ServerStatus::Running
    } else {
        ServerStatus::Stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_free_port_starts_at_minecrafts_port() {
        assert_eq!(first_free_port(&[]), Some(25_565));
    }

    #[test]
    fn first_free_port_skips_ports_in_use() {
        assert_eq!(first_free_port(&[25_565]), Some(25_566));
        assert_eq!(first_free_port(&[25_566, 25_565]), Some(25_567));
        // Taken ports below Minecraft's own are ignored.
        assert_eq!(first_free_port(&[25565 - 1, 25_565]), Some(25_566));
    }

    #[test]
    fn status_follows_the_container_state() {
        let running = ServerState {
            id: "a".to_string(),
            container: "pumapanel-a".to_string(),
            state: "running".to_string(),
            running: true,
        };
        let stopped = ServerState {
            running: false,
            state: "exited".to_string(),
            ..running.clone()
        };

        assert_eq!(status_of(&running), ServerStatus::Running);
        assert_eq!(status_of(&stopped), ServerStatus::Stopped);
    }
}
