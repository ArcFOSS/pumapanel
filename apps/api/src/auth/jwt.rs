use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::ApiError;

/// Claims carried by the session JWT. Only what the server needs to identify
/// the subject is included.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user id.
    pub sub: String,
    /// Issuer.
    pub iss: String,
    /// Issued-at (Unix seconds).
    pub iat: u64,
    /// Expiry (Unix seconds).
    pub exp: u64,
}

/// Issues a signed HS256 token for `user_id`.
pub fn issue_token(config: &Config, user_id: &str) -> Result<String, ApiError> {
    let now = now_secs();
    let claims = Claims {
        sub: user_id.to_string(),
        iss: config.jwt_issuer.clone(),
        iat: now,
        exp: now.saturating_add(config.jwt_expiration.as_secs()),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|error| ApiError::internal(format!("failed to encode token: {error}")))
}

/// Verifies a token's signature, algorithm, issuer, expiry, and required claims.
///
/// Any failure is reported as [`ApiError::Unauthorized`] so callers cannot
/// distinguish the reason. Decoded-but-unverified claims are never returned.
pub fn verify_token(config: &Config, token: &str) -> Result<Claims, ApiError> {
    // `Validation::new` pins the accepted algorithm to HS256 and enables expiry
    // checks; the token's own `alg` header is never trusted.
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[config.jwt_issuer.as_str()]);
    validation.set_required_spec_claims(&["exp", "sub", "iss"]);

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| ApiError::Unauthorized)?;

    Ok(data.claims)
}

/// Current time as Unix seconds.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
