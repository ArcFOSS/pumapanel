//! HTTP client for the PumaPanel agent.
//!
//! The agent is the only component that manages containers, so the API reaches
//! it over a small token-authenticated HTTP API. The wire types come from the
//! `agent` crate, so both sides share one definition of the protocol instead of
//! drifting apart.
//!
//! Agent failures are translated into [`ApiError`]s the frontend can act on:
//! a rejected command becomes a 400 carrying the runtime's own message, an
//! unreachable agent becomes a 503, and anything unintelligible a 502.

use std::time::Duration;

use agent::wire::{CreateServerRequest, ErrorBody, RuntimeInfo, ServerList, ServerState};
use reqwest::{Client, StatusCode, Url};
use serde::de::DeserializeOwned;

use crate::error::ApiError;

/// Client for one PumaPanel agent.
#[derive(Clone)]
pub struct AgentClient {
    http: Client,
    base_url: Url,
    token: String,
}

impl AgentClient {
    /// Builds a client for `base_url`, authenticating with `token`.
    pub fn new(base_url: &str, token: &str, timeout: Duration) -> Result<Self, ApiError> {
        let mut base_url = Url::parse(base_url).map_err(|error| {
            ApiError::Unavailable(format!("AGENT_URL is not a valid URL: {error}"))
        })?;

        // Join relative paths onto the base *directory*, so an agent mounted
        // under a sub-path (`http://host/agent`) keeps that prefix instead of
        // having its last segment replaced by `Url::join`.
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }

        let http = Client::builder().timeout(timeout).build().map_err(|error| {
            ApiError::Unavailable(format!("Could not build the agent HTTP client: {error}"))
        })?;

        Ok(Self {
            http,
            base_url,
            token: token.to_string(),
        })
    }

    /// Reports the runtime the agent will use to run servers.
    pub async fn runtime(&self) -> Result<RuntimeInfo, ApiError> {
        self.get("runtime").await
    }

    /// Lists the containers the agent currently manages.
    pub async fn list(&self) -> Result<ServerList, ApiError> {
        self.get("servers").await
    }

    /// Creates and starts a server container.
    pub async fn create(&self, request: &CreateServerRequest) -> Result<ServerState, ApiError> {
        let response = self
            .http
            .post(self.url("servers")?)
            .bearer_auth(&self.token)
            .json(request)
            .send()
            .await
            .map_err(|error| self.transport(error))?;

        self.decode(response).await
    }

    /// Starts a stopped server.
    pub async fn start(&self, id: &str) -> Result<ServerState, ApiError> {
        self.action(id, "start").await
    }

    /// Stops a running server.
    pub async fn stop(&self, id: &str) -> Result<ServerState, ApiError> {
        self.action(id, "stop").await
    }

    /// Removes a server container, optionally with its data volume.
    ///
    /// A container that is already gone counts as success, so retrying a delete
    /// that partially failed is always safe.
    pub async fn remove(&self, id: &str, remove_data: bool) -> Result<(), ApiError> {
        let response = self
            .http
            .delete(self.url(&format!("servers/{id}"))?)
            .bearer_auth(&self.token)
            .query(&[("removeData", remove_data)])
            .send()
            .await
            .map_err(|error| self.transport(error))?;

        if response.status().is_success() || response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        Err(self.failure(response).await)
    }

    async fn action(&self, id: &str, verb: &str) -> Result<ServerState, ApiError> {
        let response = self
            .http
            .post(self.url(&format!("servers/{id}/{verb}"))?)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| self.transport(error))?;

        self.decode(response).await
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let response = self
            .http
            .get(self.url(path)?)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| self.transport(error))?;

        self.decode(response).await
    }

    async fn decode<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T, ApiError> {
        if !response.status().is_success() {
            return Err(self.failure(response).await);
        }

        response.json::<T>().await.map_err(|error| {
            ApiError::Upstream(format!("The agent returned an unreadable response: {error}"))
        })
    }

    /// Turns a failed agent response into an API error, preserving the agent's
    /// own message when it sent one.
    async fn failure(&self, response: reqwest::Response) -> ApiError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<ErrorBody>(&body)
            .map(|body| body.error.message)
            .unwrap_or_else(|_| body.trim().to_string());

        map_status(status, &message)
    }

    /// A connection that never produced a response: the usual cause is that the
    /// agent is not running, or is bound somewhere the panel cannot reach.
    fn transport(&self, error: reqwest::Error) -> ApiError {
        if error.is_timeout() {
            return ApiError::Unavailable(format!(
                "The agent at {} did not respond in time.",
                self.base_url
            ));
        }

        ApiError::Unavailable(format!(
            "Could not reach the agent at {}: {error}",
            self.base_url
        ))
    }

    fn url(&self, path: &str) -> Result<Url, ApiError> {
        self.base_url
            .join(path)
            .map_err(|error| ApiError::Unavailable(format!("Invalid agent URL: {error}")))
    }
}

/// Maps an agent status code onto the API's error model.
fn map_status(status: StatusCode, message: &str) -> ApiError {
    match status {
        // The request caused these (a bound port, an unknown version, a
        // duplicate container), so the runtime's message is the useful one.
        StatusCode::BAD_REQUEST | StatusCode::CONFLICT | StatusCode::UNPROCESSABLE_ENTITY => {
            ApiError::Runtime(message.to_string())
        }
        // The panel is misconfigured (usually a mismatched AGENT_TOKEN).
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ApiError::Unavailable(format!(
            "The panel is not allowed to use the agent: {message}"
        )),
        // The agent has no container runtime to run servers with.
        StatusCode::SERVICE_UNAVAILABLE => ApiError::Unavailable(message.to_string()),
        // The panel knows this server but the host does not: an inconsistency
        // worth surfacing rather than hiding.
        StatusCode::NOT_FOUND => ApiError::Upstream(format!(
            "The host no longer has a container for this server: {message}"
        )),
        other => ApiError::Upstream(format!("The agent failed with {other}: {message}")),
    }
}
