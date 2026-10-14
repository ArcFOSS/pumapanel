use std::sync::Arc;

use sqlx::SqlitePool;

use crate::agent_client::AgentClient;
use crate::config::Config;
use crate::error::ApiError;
use crate::paper::PaperClient;

/// Shared, cheaply-clonable application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    /// Client for the agent that manages containers, or `None` when server
    /// management is not configured for this deployment. Handlers that need it
    /// return a 503 rather than failing to start.
    pub agent: Option<AgentClient>,
    /// PaperMC's version catalogue, cached in memory.
    pub paper: PaperClient,
}

impl AppState {
    /// Builds the shared state and the HTTP clients it depends on.
    ///
    /// The only way this fails is a malformed `AGENT_URL` or `PAPER_API_URL`,
    /// which is a deployment mistake worth refusing to start over.
    pub fn new(db: SqlitePool, config: Config) -> Result<Self, ApiError> {
        // Both halves of the agent configuration are validated together in
        // `Config`, so one being present means the other is too.
        let agent = match (&config.agent_url, &config.agent_token) {
            (Some(url), Some(token)) => Some(AgentClient::new(url, token, config.agent_timeout)?),
            _ => None,
        };
        let paper = PaperClient::new(&config)?;

        Ok(Self {
            db,
            config: Arc::new(config),
            agent,
            paper,
        })
    }
}
