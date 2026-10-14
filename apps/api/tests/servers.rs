//! Integration tests for the server-management API.
//!
//! These drive the real stack: the API's router, its agent HTTP client, a *real*
//! agent router served over loopback, and a stub PaperMC API. Only Docker/Podman
//! and PaperMC itself are replaced, so the wire contract between the API and the
//! agent — request shapes, status codes, error bodies — is pinned by these tests.

mod common;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent::command::Invocation;
use agent::config::Config as AgentConfig;
use agent::detect::{Detection, Resolved};
use agent::engine::{ContainerEngine, EngineError, RunOutput};
use agent::operations::Operations;
use agent::testing::{StubEngine, inspect_json};
use agent::wire::{HostEnvironment, Runtime};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
// Aliased so it does not shadow `common::get`, the request builder.
use axum::routing::get as route;
use serde_json::json;
use sqlx::SqlitePool;

use common::*;

const PASSWORD: &str = "password123";
const AGENT_TOKEN: &str = "agent-token-0123456789abcdef0123456789abcdef";

/// A trimmed Fill v3 payload: two releases, one pre-release, and the minimum
/// Java each release needs.
const PAPER_VERSIONS: &str = r#"{
    "project": { "id": "paper", "name": "Paper" },
    "versions": [
        { "version": { "id": "1.21.8", "support": { "status": "SUPPORTED" },
            "java": { "version": { "minimum": 21 } } }, "builds": [60] },
        { "version": { "id": "1.21.11-rc1", "support": { "status": "UNSUPPORTED" },
            "java": { "version": { "minimum": 21 } } }, "builds": [1] },
        { "version": { "id": "1.7.10", "support": { "status": "UNSUPPORTED" },
            "java": { "version": { "minimum": 6 } } }, "builds": [1] }
    ]
}"#;

// --------------------------------------------------------------------------
// A fake container runtime
// --------------------------------------------------------------------------

/// One container the fake runtime knows about.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FakeContainer {
    id: String,
    running: bool,
}

/// A stateful stand-in for Docker/Podman.
///
/// It answers the same commands the agent issues against a real runtime, keeping
/// just enough state that create/start/stop/delete behave believably. It can be
/// told to fail the next `run`, to exercise the API's rollback path.
struct FakeRuntime {
    containers: Arc<Mutex<Vec<FakeContainer>>>,
    fail_create: Arc<AtomicBool>,
    engine: Arc<StubEngine>,
}

impl FakeRuntime {
    fn new() -> Self {
        let containers = Arc::new(Mutex::new(Vec::new()));
        let fail_create = Arc::new(AtomicBool::new(false));
        let state = Arc::clone(&containers);
        let flag = Arc::clone(&fail_create);
        let engine = Arc::new(StubEngine::with_responder(move |invocation| {
            respond(&state, &flag, invocation)
        }));

        Self {
            containers,
            fail_create,
            engine,
        }
    }

    fn containers(&self) -> Vec<FakeContainer> {
        self.containers.lock().expect("container lock").clone()
    }

    /// The runtime subcommand of every command the agent ran, in order.
    fn subcommands(&self) -> Vec<String> {
        self.engine.subcommands()
    }

    /// Makes the runtime refuse to create containers, as it would for a port
    /// that is already bound on the host.
    fn fail_create(&self) {
        self.fail_create.store(true, Ordering::SeqCst);
    }
}

fn container_name_of(id: &str) -> String {
    format!("pumapanel-{id}")
}

fn respond(
    containers: &Mutex<Vec<FakeContainer>>,
    fail_create: &AtomicBool,
    invocation: &Invocation,
) -> Result<RunOutput, EngineError> {
    let mut containers = containers.lock().expect("container lock");
    let subcommand = invocation.args.first().cloned().unwrap_or_default();

    match subcommand.as_str() {
        "run" if fail_create.load(Ordering::SeqCst) => Ok(RunOutput::failed(
            "Error: address already in use",
        )),
        "run" => {
            let id = label_value(&invocation.args, "pumapanel.server").unwrap_or_default();
            containers.push(FakeContainer { id, running: true });
            Ok(RunOutput::ok("container-id"))
        }
        "start" | "stop" => {
            let name = invocation.args.last().cloned().unwrap_or_default();
            let running = subcommand == "start";
            match containers
                .iter_mut()
                .find(|container| container_name_of(&container.id) == name)
            {
                Some(container) => {
                    container.running = running;
                    Ok(RunOutput::ok(name))
                }
                None => Ok(RunOutput::failed(format!("Error: no such container: {name}"))),
            }
        }
        "inspect" => {
            let name = invocation.args.get(1).cloned().unwrap_or_default();
            match containers
                .iter()
                .find(|container| container_name_of(&container.id) == name)
            {
                Some(container) => Ok(RunOutput::ok(inspect_json(
                    &container.id,
                    container.running,
                    state_of(container.running),
                ))),
                None => Ok(RunOutput::failed(format!("Error: no such container: {name}"))),
            }
        }
        "ps" => {
            let listing = containers
                .iter()
                .map(|container| format!("{}\n", container_name_of(&container.id)))
                .collect::<String>();
            Ok(RunOutput::ok(listing))
        }
        "rm" => {
            let name = invocation.args.last().cloned().unwrap_or_default();
            let before = containers.len();
            containers.retain(|container| container_name_of(&container.id) != name);

            if containers.len() == before {
                Ok(RunOutput::failed(format!("Error: no such container: {name}")))
            } else {
                Ok(RunOutput::ok(""))
            }
        }
        "volume" => Ok(RunOutput::ok("")),
        "--version" => Ok(RunOutput::ok("podman version 6.1.3")),
        _ => Ok(RunOutput::ok("")),
    }
}

fn label_value(args: &[String], key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(str::to_string))
}

fn state_of(running: bool) -> &'static str {
    if running { "running" } else { "exited" }
}

// --------------------------------------------------------------------------
// Real agent and stub PaperMC servers on loopback
// --------------------------------------------------------------------------

fn ready_detection() -> Detection {
    Detection {
        host_environment: HostEnvironment::BareMetal,
        // A bare-metal agent falls back to Podman, which is what these tests
        // expect the panel to report.
        resolved: Resolved::Ready {
            runtime: Runtime::Podman,
            binary: PathBuf::from("/usr/bin/podman"),
        },
    }
}

fn agent_config() -> AgentConfig {
    AgentConfig {
        bind: "127.0.0.1:0".to_string(),
        token: AGENT_TOKEN.to_string(),
        runtime_override: None,
        image: "itzg/minecraft-server".to_string(),
        default_memory_mb: 2048,
    }
}

/// Serves the real agent router on a loopback port, backed by `runtime`.
async fn start_agent(runtime: &FakeRuntime) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an agent port");
    let address = listener.local_addr().expect("agent address");

    let engine: Arc<dyn ContainerEngine> = runtime.engine.clone();
    let operations = Arc::new(Operations::new(engine, ready_detection(), agent_config()));
    let router = agent::routes::router(operations, Arc::from(AGENT_TOKEN));

    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    format!("http://{address}")
}

/// A stub PaperMC API that can be switched to failing mid-test, to exercise the
/// cached-versions path.
struct PaperStub {
    url: String,
    failing: Arc<AtomicBool>,
}

impl PaperStub {
    fn fail(&self) {
        self.failing.store(true, Ordering::SeqCst);
    }
}

async fn start_paper() -> PaperStub {
    let failing = Arc::new(AtomicBool::new(false));
    let handler_flag = Arc::clone(&failing);

    let app = Router::new().route(
        "/v3/projects/paper/versions",
        route(move || {
            let failing = Arc::clone(&handler_flag);
            async move {
                if failing.load(Ordering::SeqCst) {
                    (StatusCode::INTERNAL_SERVER_ERROR, "paper is down")
                } else {
                    (StatusCode::OK, PAPER_VERSIONS)
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind paper port");
    let address = listener.local_addr().expect("paper address");

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    PaperStub {
        url: format!("http://{address}/v3"),
        failing,
    }
}

/// A URL on a loopback port with nothing listening, to exercise an unreachable
/// agent.
async fn closed_url() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("reserve a port");
    let address = listener.local_addr().expect("address");
    drop(listener);

    format!("http://{address}")
}

// --------------------------------------------------------------------------
// Harness
// --------------------------------------------------------------------------

/// Everything a test needs: the app, the fake runtime, and an admin session.
struct Harness {
    ctx: TestContext,
    runtime: FakeRuntime,
    cookie: String,
}

/// Starts the agent and Paper stubs and points the API at them.
async fn setup() -> Harness {
    let runtime = FakeRuntime::new();
    let agent_url = start_agent(&runtime).await;
    let paper_url = start_paper().await.url;

    let ctx = setup_with(move |key| match key {
        "AGENT_URL" => Some(agent_url.clone()),
        "AGENT_TOKEN" => Some(AGENT_TOKEN.to_string()),
        "PAPER_API_URL" => Some(paper_url.clone()),
        _ => None,
    })
    .await;

    let cookie = admin_session(&ctx).await;

    Harness {
        ctx,
        runtime,
        cookie,
    }
}

impl Harness {
    /// Sends a request authenticated as the harness's admin.
    async fn send(&self, request: Request<Body>) -> Response {
        send(
            &self.ctx.app,
            with_header(request, header::COOKIE, self.cookie.clone()),
        )
        .await
    }

    /// Creates a server through the API and returns its id.
    async fn create_server(&self, name: &str, version: &str) -> String {
        let response = self
            .send(json_post(
                "/admin/servers",
                json!({ "name": name, "version": version }),
            ))
            .await;
        assert_eq!(
            response.status,
            StatusCode::CREATED,
            "body: {}",
            response.body
        );

        response.json()["server"]["id"]
            .as_str()
            .expect("server id")
            .to_string()
    }
}

/// Promotes an account to `admin` directly in the database.
async fn promote_to_admin(pool: &SqlitePool, id: &str) {
    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .expect("promote user to admin");
}

/// Registers the first account, promotes it, and returns its session cookie.
async fn admin_session(ctx: &TestContext) -> String {
    let registered = register(&ctx.app, "admin", "admin@example.com", PASSWORD).await;
    assert_eq!(registered.status, StatusCode::CREATED);
    promote_to_admin(&ctx.pool, &user_id(&registered.json())).await;

    session_cookie(&registered.cookies).expect("session cookie")
}

fn delete(path: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(path)
        .body(Body::empty())
        .expect("build request")
}

// --------------------------------------------------------------------------
// Version catalogue
// --------------------------------------------------------------------------

#[tokio::test]
async fn versions_are_proxied_with_prereleases_dropped() {
    let harness = setup().await;

    let response = harness.send(get("/admin/servers/versions")).await;

    assert_eq!(response.status, StatusCode::OK);
    let body = response.json();
    let versions = body["versions"].as_array().expect("versions array");
    let ids: Vec<&str> = versions
        .iter()
        .map(|version| version["version"].as_str().expect("version id"))
        .collect();

    // Newest first, and the pre-release is not offered.
    assert_eq!(ids, vec!["1.21.8", "1.7.10"]);
    assert_eq!(versions[0]["support"], "SUPPORTED");
    assert_eq!(versions[0]["javaMin"], 21);
    assert_eq!(versions[1]["javaMin"], 6);
    assert_eq!(body["stale"], false);
}

#[tokio::test]
async fn versions_fail_clearly_when_paper_is_unreachable() {
    // A cold cache against a failing upstream: there is nothing to fall back on.
    let paper = start_paper().await;
    paper.fail();
    let paper_url = paper.url.clone();
    let agent_url = start_agent(&FakeRuntime::new()).await;

    let ctx = setup_with(move |key| match key {
        "AGENT_URL" => Some(agent_url.clone()),
        "AGENT_TOKEN" => Some(AGENT_TOKEN.to_string()),
        "PAPER_API_URL" => Some(paper_url.clone()),
        _ => None,
    })
    .await;
    let cookie = admin_session(&ctx).await;

    let response = send(
        &ctx.app,
        with_header(get("/admin/servers/versions"), header::COOKIE, cookie),
    )
    .await;

    assert_eq!(response.status, StatusCode::BAD_GATEWAY);
    assert_eq!(response.json()["error"]["code"], "upstream_error");
}

#[tokio::test]
async fn versions_are_served_from_cache_when_paper_fails_the_refresh() {
    let paper = start_paper().await;
    let paper_url = paper.url.clone();
    let agent_url = start_agent(&FakeRuntime::new()).await;

    let ctx = setup_with(move |key| match key {
        "AGENT_URL" => Some(agent_url.clone()),
        "AGENT_TOKEN" => Some(AGENT_TOKEN.to_string()),
        "PAPER_API_URL" => Some(paper_url.clone()),
        // A one-second TTL, so the second request is forced to refresh.
        "PAPER_CACHE_TTL_SECS" => Some("1".to_string()),
        _ => None,
    })
    .await;
    let cookie = admin_session(&ctx).await;

    // Warm the cache.
    let first = send(
        &ctx.app,
        with_header(get("/admin/servers/versions"), header::COOKIE, cookie.clone()),
    )
    .await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.json()["stale"], false);

    // Let the cache expire, then take PaperMC down.
    tokio::time::sleep(Duration::from_millis(1_100)).await;
    paper.fail();

    let second = send(
        &ctx.app,
        with_header(get("/admin/servers/versions"), header::COOKIE, cookie),
    )
    .await;

    assert_eq!(second.status, StatusCode::OK);
    let body = second.json();
    assert_eq!(body["stale"], true, "the copy is marked out of date");
    assert_eq!(
        body["versions"].as_array().map(Vec::len),
        Some(2),
        "the cached list is still served"
    );
}

// --------------------------------------------------------------------------
// Creating servers
// --------------------------------------------------------------------------

#[tokio::test]
async fn creating_a_server_stores_it_and_starts_a_container() {
    let harness = setup().await;

    let response = harness
        .send(json_post(
            "/admin/servers",
            json!({ "name": "Survival", "version": "1.21.8" }),
        ))
        .await;

    assert_eq!(response.status, StatusCode::CREATED);
    let server = &response.json()["server"];
    assert_eq!(server["name"], "Survival");
    assert_eq!(server["version"], "1.21.8");
    assert_eq!(server["status"], "running");
    assert_eq!(server["hostPort"], 25_565);
    assert_eq!(server["memoryMb"], 2048);
    assert_eq!(server["javaMin"], 21);
    assert!(server["id"].is_string());
    assert!(server["createdAt"].is_string());

    // The runtime was asked to create exactly one container, then inspected it.
    assert_eq!(harness.runtime.subcommands(), vec!["run", "inspect"]);
    let containers = harness.runtime.containers();
    assert_eq!(containers.len(), 1);
    assert!(containers[0].running);
}

#[tokio::test]
async fn creating_allocates_the_next_free_port_per_server() {
    let harness = setup().await;

    harness.create_server("First", "1.21.8").await;
    let second = harness
        .send(json_post(
            "/admin/servers",
            json!({ "name": "Second", "version": "1.21.8" }),
        ))
        .await;

    // The second server is not published on the first one's port.
    assert_eq!(second.json()["server"]["hostPort"], 25_566);
    assert_eq!(harness.runtime.containers().len(), 2);
}

#[tokio::test]
async fn creating_a_server_rejects_an_unknown_version() {
    let harness = setup().await;

    let response = harness
        .send(json_post(
            "/admin/servers",
            json!({ "name": "Survival", "version": "1.2.3" }),
        ))
        .await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    let body = response.json();
    assert_eq!(body["error"]["code"], "validation_error");
    assert_eq!(body["error"]["fields"][0]["field"], "version");
    // Nothing was stored, and no container was created.
    assert!(harness.runtime.containers().is_empty());
}

#[tokio::test]
async fn creating_a_server_reports_every_invalid_field_at_once() {
    let harness = setup().await;

    let response = harness
        .send(json_post(
            "/admin/servers",
            json!({ "name": "", "memoryMb": 1, "hostPort": 80 }),
        ))
        .await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    let fields: Vec<String> = response.json()["error"]["fields"]
        .as_array()
        .expect("field errors")
        .iter()
        .map(|field| field["field"].as_str().unwrap_or_default().to_string())
        .collect();

    assert_eq!(fields, vec!["name", "version", "memoryMb", "hostPort"]);
}

#[tokio::test]
async fn creating_a_server_rejects_a_port_already_in_use() {
    let harness = setup().await;
    harness.create_server("First", "1.21.8").await;

    let response = harness
        .send(json_post(
            "/admin/servers",
            json!({ "name": "Second", "version": "1.21.8", "hostPort": 25565 }),
        ))
        .await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
    assert_eq!(response.json()["error"]["fields"][0]["field"], "hostPort");
}

#[tokio::test]
async fn a_runtime_failure_during_create_rolls_the_record_back() {
    let harness = setup().await;
    harness.runtime.fail_create();

    let response = harness
        .send(json_post(
            "/admin/servers",
            json!({ "name": "Survival", "version": "1.21.8" }),
        ))
        .await;

    // The agent refuses the command, and the panel reports the runtime's own
    // message rather than claiming success.
    assert_eq!(response.status, StatusCode::BAD_GATEWAY);
    let body = response.json();
    assert_eq!(body["error"]["code"], "upstream_error");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("address already in use"),
        "the runtime's own message should survive: {body}"
    );

    // The panel keeps no phantom server the runtime never created.
    let listed = harness.send(get("/admin/servers")).await;
    assert_eq!(listed.json()["servers"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn creating_a_server_requires_an_agent() {
    let ctx = common::setup().await;
    let cookie = admin_session(&ctx).await;

    let response = send(
        &ctx.app,
        with_header(
            json_post(
                "/admin/servers",
                json!({ "name": "Survival", "version": "1.21.8" }),
            ),
            header::COOKIE,
            cookie,
        ),
    )
    .await;

    assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.json()["error"]["code"], "runtime_unavailable");
}

// --------------------------------------------------------------------------
// Listing, starting, stopping and deleting
// --------------------------------------------------------------------------

#[tokio::test]
async fn listing_merges_records_with_live_state_and_the_runtime() {
    let harness = setup().await;
    let id = harness.create_server("Survival", "1.21.8").await;

    let response = harness.send(get("/admin/servers")).await;
    assert_eq!(response.status, StatusCode::OK);

    let body = response.json();
    let servers = body["servers"].as_array().expect("servers array");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0]["id"], json!(id));
    assert_eq!(servers[0]["name"], "Survival");
    assert_eq!(servers[0]["status"], "running");

    // The page learns the runtime in the same round trip.
    assert_eq!(body["runtime"]["runtime"], "podman");
    assert_eq!(body["runtime"]["available"], true);
    assert_eq!(body["runtime"]["hostEnvironment"], "bare-metal");

    // Stopping is reflected on the next read.
    let stopped = harness
        .send(json_post(&format!("/admin/servers/{id}/stop"), json!({})))
        .await;
    assert_eq!(stopped.status, StatusCode::OK);
    assert_eq!(stopped.json()["server"]["status"], "stopped");

    let listed = harness.send(get("/admin/servers")).await;
    assert_eq!(listed.json()["servers"][0]["status"], "stopped");

    // Starting brings it back.
    let started = harness
        .send(json_post(&format!("/admin/servers/{id}/start"), json!({})))
        .await;
    assert_eq!(started.json()["server"]["status"], "running");

    // Only the state-changing commands are asserted: reading the list also asks
    // the agent for its runtime version, and those two requests run
    // concurrently, so their relative order is not fixed.
    let mutations: Vec<String> = harness
        .runtime
        .subcommands()
        .into_iter()
        .filter(|command| matches!(command.as_str(), "run" | "start" | "stop" | "rm" | "volume"))
        .collect();
    assert_eq!(mutations, vec!["run", "stop", "start"]);
}

#[tokio::test]
async fn deleting_a_server_removes_its_container_and_data() {
    let harness = setup().await;
    let id = harness.create_server("Survival", "1.21.8").await;

    let response = harness
        .send(delete(&format!("/admin/servers/{id}?removeData=true")))
        .await;

    assert_eq!(response.status, StatusCode::NO_CONTENT);
    assert!(harness.runtime.containers().is_empty());
    assert_eq!(
        harness.runtime.subcommands(),
        vec!["run", "inspect", "rm", "volume"]
    );

    // The panel record is gone too.
    let listed = harness.send(get("/admin/servers")).await;
    assert_eq!(listed.json()["servers"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn deleting_a_server_keeps_its_data_by_default() {
    let harness = setup().await;
    let id = harness.create_server("Survival", "1.21.8").await;

    let response = harness.send(delete(&format!("/admin/servers/{id}"))).await;

    assert_eq!(response.status, StatusCode::NO_CONTENT);
    assert_eq!(harness.runtime.subcommands(), vec!["run", "inspect", "rm"]);
}

#[tokio::test]
async fn deleting_an_unknown_server_is_not_found() {
    let harness = setup().await;

    let response = harness.send(delete("/admin/servers/nope")).await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.json()["error"]["code"], "not_found");
}

#[tokio::test]
async fn acting_on_an_unknown_server_is_not_found() {
    let harness = setup().await;

    let start = harness
        .send(json_post("/admin/servers/nope/start", json!({})))
        .await;

    assert_eq!(start.status, StatusCode::NOT_FOUND);
}

// --------------------------------------------------------------------------
// Degraded deployments and authorisation
// --------------------------------------------------------------------------

#[tokio::test]
async fn without_an_agent_the_page_explains_itself() {
    let ctx = common::setup().await;
    let cookie = admin_session(&ctx).await;

    let response = send(
        &ctx.app,
        with_header(get("/admin/servers"), header::COOKIE, cookie.clone()),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    let body = response.json();
    assert_eq!(body["runtime"]["available"], false);
    assert!(body["runtime"]["runtime"].is_null());
    assert!(
        body["runtime"]["message"]
            .as_str()
            .expect("a reason")
            .contains("agent"),
        "the message should explain what is missing"
    );

    // Mutations fail clearly instead of pretending to work.
    let created = send(
        &ctx.app,
        with_header(
            json_post(
                "/admin/servers",
                json!({ "name": "Survival", "version": "1.21.8" }),
            ),
            header::COOKIE,
            cookie,
        ),
    )
    .await;

    assert_eq!(created.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(created.json()["error"]["code"], "runtime_unavailable");
}

#[tokio::test]
async fn an_unreachable_agent_leaves_statuses_unknown() {
    let paper_url = start_paper().await.url;
    let agent_url = closed_url().await;

    let ctx = setup_with(move |key| match key {
        "AGENT_URL" => Some(agent_url.clone()),
        "AGENT_TOKEN" => Some(AGENT_TOKEN.to_string()),
        "PAPER_API_URL" => Some(paper_url.clone()),
        _ => None,
    })
    .await;
    let cookie = admin_session(&ctx).await;

    // A record exists, but the host cannot be asked about it.
    sqlx::query(
        "INSERT INTO servers (id, name, version, memory_mb, host_port, java_min) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind("orphan")
    .bind("Orphan")
    .bind("1.21.8")
    .bind(2048)
    .bind(25_565)
    .bind(21)
    .execute(&ctx.pool)
    .await
    .expect("insert server");

    let response = send(
        &ctx.app,
        with_header(get("/admin/servers"), header::COOKIE, cookie),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    let body = response.json();
    assert_eq!(body["servers"][0]["status"], "unknown");
    assert_eq!(body["runtime"]["available"], false);
    assert!(
        body["runtime"]["message"]
            .as_str()
            .expect("a reason")
            .contains("agent"),
        "the runtime view should name the agent as the problem"
    );
}

#[tokio::test]
async fn server_routes_require_an_admin() {
    let harness = setup().await;

    // No session at all.
    let anonymous = send(&harness.ctx.app, get("/admin/servers")).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);

    // A signed-in account that is not an admin.
    let registered = register(&harness.ctx.app, "plainuser", "plain@example.com", PASSWORD).await;
    let cookie = session_cookie(&registered.cookies).expect("session cookie");

    let forbidden = send(
        &harness.ctx.app,
        with_header(get("/admin/servers"), header::COOKIE, cookie.clone()),
    )
    .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    let forbidden_create = send(
        &harness.ctx.app,
        with_header(
            json_post(
                "/admin/servers",
                json!({ "name": "Survival", "version": "1.21.8" }),
            ),
            header::COOKIE,
            cookie,
        ),
    )
    .await;
    assert_eq!(forbidden_create.status, StatusCode::FORBIDDEN);
    assert!(harness.runtime.containers().is_empty());
}
