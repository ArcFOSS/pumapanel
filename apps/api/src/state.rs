use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;

/// Shared, cheaply-clonable application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
}
