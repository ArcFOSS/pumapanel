use serde::Serialize;
use sqlx::SqlitePool;

/// Persisted user record. `password_hash` never leaves the backend.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: String,
}

/// User data that is safe to return to clients.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

/// Applies the project's email policy: trim surrounding whitespace and
/// lowercase. Used consistently before validation and storage so that
/// `User@Example.com` and `user@example.com` are the same account.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Inserts a new user. Returns the stored row.
///
/// A duplicate email surfaces as a unique-constraint violation, which callers
/// translate into [`crate::error::ApiError::EmailTaken`].
pub async fn create(
    pool: &SqlitePool,
    email: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, password_hash) \
         VALUES (?1, ?2, ?3) \
         RETURNING id, email, password_hash, created_at",
    )
    .bind(&id)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

pub async fn find_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE email = ?1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}
