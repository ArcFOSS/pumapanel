//! End-to-end tests for the agent's HTTP API.
//!
//! The router, handlers, parsing and error mapping are all real; only the
//! container runtime is replaced by [`StubEngine`]. That means these tests pin
//! the wire contract the main API depends on, without needing Docker or Podman.

use std::path::PathBuf;
use std::sync::Arc;

use agent::command::container_name;
use agent::config::Config;
use agent::detect::{Detection, Resolved};
use agent::engine::RunOutput;
use agent::operations::Operations;
use agent::routes;
use agent::testing::{StubEngine, inspect_json};
use agent::wire::{HostEnvironment, Runtime};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";
const ID: &str = "6f9619ff-816d-4d01-b42d-00cf4fc964ff";

fn config() -> Config {
    Config {
        bind: "127.0.0.1:9091".to_string(),
        token: TOKEN.to_string(),
        runtime_override: None,
        image: "itzg/minecraft-server".to_string(),
        default_memory_mb: 2048,
    }
}

fn detection(runtime: Option<Runtime>) -> Detection {
    let resolved = match runtime {
        Some(runtime) => Resolved::Ready {
            runtime,
            binary: PathBuf::from("/usr/bin/podman"),
        },
        None => Resolved::Unavailable {
            runtime: None,
            message: "No container runtime found.".to_string(),
        },
    };
    Detection {
        host_environment: HostEnvironment::BareMetal,
        resolved,
    }
}

fn app(engine: Arc<StubEngine>) -> Router {
    app_with(engine, detection(Some(Runtime::Podman)))
}

fn app_with(engine: Arc<StubEngine>, detection: Detection) -> Router {
    let operations = Arc::new(Operations::new(engine, detection, config()));
    routes::router(operations, Arc::from(TOKEN))
}

/// Sends a request through the real router and returns the status and body.
async fn send(app: Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.oneshot(request).await.expect("router responded");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("readable body");
    (status, String::from_utf8_lossy(&body).into_owned())
}

fn authenticated(method: &str, path: &str) -> axum::http::request::Builder {
    Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {TOKEN}"))
}

fn get(path: &str) -> Request<Body> {
    authenticated("GET", path).body(Body::empty()).expect("request")
}

fn post(path: &str, body: &str) -> Request<Body> {
    authenticated("POST", path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

fn delete(path: &str) -> Request<Body> {
    authenticated("DELETE", path).body(Body::empty()).expect("request")
}

fn create_body() -> String {
    serde_json::json!({
        "id": ID,
        "version": "1.21.8",
        "memoryMb": 4096,
        "hostPort": 25566,
        "javaMin": 21,
    })
    .to_string()
}

/// The stub answers `run` with success and `inspect` with a running container.
fn running_engine() -> Arc<StubEngine> {
    Arc::new(StubEngine::with_responder(|invocation| {
        if invocation.args.first().map(String::as_str) == Some("inspect") {
            return Ok(RunOutput::ok(inspect_json(ID, true, "running")));
        }
        Ok(RunOutput::ok(""))
    }))
}

#[tokio::test]
async fn health_is_reachable_without_a_token() {
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("request");

    let (status, _) = send(app(Arc::new(StubEngine::new())), request).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn protected_routes_require_the_token() {
    let unauthenticated = Request::builder()
        .uri("/runtime")
        .body(Body::empty())
        .expect("request");
    let wrong_token = Request::builder()
        .uri("/runtime")
        .header("authorization", "Bearer not-the-token")
        .body(Body::empty())
        .expect("request");

    for request in [unauthenticated, wrong_token] {
        let (status, body) = send(app(Arc::new(StubEngine::new())), request).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("unauthorized"), "body was {body}");
    }
}

#[tokio::test]
async fn runtime_reports_the_detected_engine() {
    let engine = Arc::new(StubEngine::with_responder(|_| {
        Ok(RunOutput::ok("podman version 5.6.1"))
    }));

    let (status, body) = send(app(engine), get("/runtime")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"runtime\":\"podman\""), "body was {body}");
    assert!(
        body.contains("\"hostEnvironment\":\"bare-metal\""),
        "body was {body}"
    );
    assert!(body.contains("\"available\":true"), "body was {body}");
}

#[tokio::test]
async fn runtime_reports_a_missing_runtime_without_failing() {
    let (status, body) = send(
        app_with(Arc::new(StubEngine::new()), detection(None)),
        get("/runtime"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"available\":false"), "body was {body}");
    assert!(body.contains("\"runtime\":null"), "body was {body}");
}

#[tokio::test]
async fn create_returns_the_new_server_and_runs_the_container() {
    let engine = running_engine();

    let (status, body) = send(app(engine.clone()), post("/servers", &create_body())).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.contains(&format!("\"id\":\"{ID}\"")), "body was {body}");
    assert!(body.contains("\"running\":true"), "body was {body}");

    assert_eq!(engine.subcommands(), vec!["run", "inspect"]);
    let created = &engine.invocations()[0];
    assert!(created.args.contains(&"TYPE=PAPER".to_string()));
    assert!(created.args.contains(&"VERSION=1.21.8".to_string()));
    assert!(created
        .args
        .contains(&"itzg/minecraft-server:java21".to_string()));
}

#[tokio::test]
async fn create_surfaces_runtime_failures() {
    let engine = Arc::new(StubEngine::with_responder(|_| {
        Ok(RunOutput::failed("Error: port is already allocated"))
    }));

    let (status, body) = send(app(engine), post("/servers", &create_body())).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body.contains("port is already allocated"), "body was {body}");
}

#[tokio::test]
async fn create_is_unavailable_without_a_runtime() {
    let (status, body) = send(
        app_with(Arc::new(StubEngine::new()), detection(None)),
        post("/servers", &create_body()),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains("runtime_unavailable"), "body was {body}");
}

#[tokio::test]
async fn list_returns_managed_containers() {
    let name = container_name(ID);
    let listed = name.clone();
    let engine = Arc::new(StubEngine::with_responder(move |invocation| {
        if invocation.args.first().map(String::as_str) == Some("ps") {
            return Ok(RunOutput::ok(format!("{listed}\n")));
        }
        Ok(RunOutput::ok(inspect_json(ID, false, "exited")))
    }));

    let (status, body) = send(app(engine), get("/servers")).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(&name), "body was {body}");
    assert!(body.contains("\"state\":\"exited\""), "body was {body}");
    assert!(body.contains("\"running\":false"), "body was {body}");
}

#[tokio::test]
async fn start_and_stop_transition_the_container() {
    let engine = running_engine();

    let (started, _) = send(
        app(engine.clone()),
        post(&format!("/servers/{ID}/start"), ""),
    )
    .await;
    let (stopped, _) = send(
        app(engine.clone()),
        post(&format!("/servers/{ID}/stop"), ""),
    )
    .await;

    assert_eq!(started, StatusCode::OK);
    assert_eq!(stopped, StatusCode::OK);
    assert_eq!(
        engine.subcommands(),
        vec!["start", "inspect", "stop", "inspect"]
    );
}

#[tokio::test]
async fn delete_removes_the_container_and_its_data_when_asked() {
    let engine = Arc::new(StubEngine::new());

    let (status, _) = send(
        app(engine.clone()),
        delete(&format!("/servers/{ID}?removeData=true")),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);

    // The container name must be prefixed exactly once.
    let name = container_name(ID);
    let invocations = engine.invocations();
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].args, vec!["rm", "--force", name.as_str()]);
    assert_eq!(invocations[1].args, vec!["volume", "rm", name.as_str()]);
}

#[tokio::test]
async fn delete_keeps_the_data_volume_by_default() {
    let engine = Arc::new(StubEngine::new());

    let (status, _) = send(app(engine.clone()), delete(&format!("/servers/{ID}"))).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(engine.subcommands(), vec!["rm"]);
}

#[tokio::test]
async fn deleting_an_unknown_server_is_idempotent() {
    let engine = Arc::new(StubEngine::with_responder(|_| {
        Ok(RunOutput::failed("Error: No such container: pumapanel-x"))
    }));

    let (status, _) = send(
        app(engine),
        delete(&format!("/servers/{ID}?removeData=true")),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);
}
