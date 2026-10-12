use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

/// Migrations are embedded into the binary at compile time, so deployments do
/// not need to ship the `migrations/` directory or run a separate migrate step.
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// A boxed error that is safe to move across threads and async tasks.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Opens the SQLite connection pool and applies any pending migrations.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, BoxError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        // Create the database file on first run instead of failing.
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}
