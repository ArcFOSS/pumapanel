use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// An account's authorization role. `Admin` grants access to the admin API.
///
/// Stored as a `TEXT` column (`user`/`admin`) by the `0003_add_role` migration;
/// `#[sqlx(rename_all = "lowercase")]` matches the column values and
/// `#[serde(rename_all = "lowercase")]` produces the same form on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }
}

/// Persisted user record. `password_hash` never leaves the backend.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub role: Role,
    pub password_hash: String,
    pub created_at: String,
}

/// User data that is safe to return to clients.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    pub id: String,
    pub email: String,
    pub username: String,
    pub role: Role,
    pub created_at: String,
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

/// Shared account policy: trim surrounding whitespace and lowercase, so that
/// `User@Example.com` / `Alice` and `user@example.com` / `alice` are the same
/// account.
fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Normalizes an email address before validation and storage.
pub fn normalize_email(email: &str) -> String {
    normalize(email)
}

/// Normalizes a username before validation and storage.
pub fn normalize_username(username: &str) -> String {
    normalize(username)
}

/// Normalizes a login identifier (an email address or a username). Both stored
/// columns use the same policy, so one normalized value can match either.
pub fn normalize_identifier(identifier: &str) -> String {
    normalize(identifier)
}

/// Returns true when a (normalized) username uses only letters, digits, and
/// underscores — the character policy enforced at registration and when an
/// admin edits a username. Length is validated separately.
pub fn has_valid_username_chars(username: &str) -> bool {
    username
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// Inserts a new user. Returns the stored row.
///
/// A duplicate email or username surfaces as a unique-constraint violation,
/// which callers translate into [`crate::error::ApiError::EmailTaken`] or
/// [`crate::error::ApiError::UsernameTaken`]. The role defaults to `user` via
/// the column default.
pub async fn create(
    pool: &SqlitePool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, username, email, password_hash) \
         VALUES (?1, ?2, ?3, ?4) \
         RETURNING id, email, username, role, password_hash, created_at",
    )
    .bind(&id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

pub async fn find_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, username, role, password_hash, created_at FROM users WHERE email = ?1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}

/// Finds an account by email address *or* username. The caller passes a value
/// already normalized with [`normalize_identifier`]; since a username cannot
/// contain `@`, at most one account can match.
pub async fn find_by_identifier(
    pool: &SqlitePool,
    identifier: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, username, role, password_hash, created_at FROM users \
         WHERE email = ?1 OR username = ?1",
    )
    .bind(identifier)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, username, role, password_hash, created_at FROM users WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Applies an administrator's edits to a user. Provided fields overwrite the
/// stored values; omitted fields keep their current value.
///
/// Returns [`sqlx::Error::RowNotFound`] when no user has this id, and a
/// unique-constraint violation when the new username or email is already in
/// use.
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    username: Option<String>,
    email: Option<String>,
    role: Option<Role>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET \
         username = COALESCE(?1, username), \
         email = COALESCE(?2, email), \
         role = COALESCE(?3, role) \
         WHERE id = ?4 \
         RETURNING id, email, username, role, password_hash, created_at",
    )
    .bind(username)
    .bind(email)
    .bind(role)
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(sqlx::Error::RowNotFound)
}