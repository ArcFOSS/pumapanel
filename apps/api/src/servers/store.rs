//! Persistence for Minecraft server metadata.
//!
//! Only the panel's own record of a server lives here. The container belongs to
//! the agent, which derives its state from container labels; `id` is the join
//! key between the two, naming both the container and its data volume
//! (`pumapanel-<id>`).

use sqlx::SqlitePool;

/// A stored server, as it appears in the `servers` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub memory_mb: u32,
    pub host_port: u16,
    pub java_min: Option<u8>,
    pub created_at: String,
}

/// The fields the panel chooses when creating a server. The `id` is generated,
/// and the timestamp is left to the database.
pub struct NewServer<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub memory_mb: u32,
    pub host_port: u16,
    pub java_min: Option<u8>,
}

/// Every server, oldest first, so the list is stable between requests.
pub async fn list(pool: &SqlitePool) -> Result<Vec<ServerRecord>, sqlx::Error> {
    sqlx::query_as::<_, ServerRecord>(
        "SELECT id, name, version, memory_mb, host_port, java_min, created_at \
         FROM servers ORDER BY created_at, id",
    )
    .fetch_all(pool)
    .await
}

pub async fn find(pool: &SqlitePool, id: &str) -> Result<Option<ServerRecord>, sqlx::Error> {
    sqlx::query_as::<_, ServerRecord>(
        "SELECT id, name, version, memory_mb, host_port, java_min, created_at \
         FROM servers WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// The ports already handed to a server, used to pick a free one.
pub async fn used_ports(pool: &SqlitePool) -> Result<Vec<u16>, sqlx::Error> {
    sqlx::query_scalar::<_, u16>("SELECT host_port FROM servers")
        .fetch_all(pool)
        .await
}

/// Inserts a server and returns it as stored.
///
/// The `host_port` column is `UNIQUE`, so a request that races another for the
/// same port fails the constraint rather than publishing two servers on it.
pub async fn insert(pool: &SqlitePool, server: &NewServer<'_>) -> Result<ServerRecord, sqlx::Error> {
    sqlx::query_as::<_, ServerRecord>(
        "INSERT INTO servers (id, name, version, memory_mb, host_port, java_min) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         RETURNING id, name, version, memory_mb, host_port, java_min, created_at",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(server.name)
    .bind(server.version)
    .bind(server.memory_mb)
    .bind(server.host_port)
    .bind(server.java_min)
    .fetch_one(pool)
    .await
}

/// Removes a server, reporting whether a row was actually deleted.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM servers WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
