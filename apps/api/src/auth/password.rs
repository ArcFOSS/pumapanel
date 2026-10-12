use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

/// Argon2id parameters (OWASP-recommended baseline): 19 MiB memory, 2
/// iterations, 1 lane. Applied to every hash so verification is deterministic.
fn argon2() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

/// Hashes a plaintext password with Argon2id and a random per-password salt.
/// The returned string is a PHC-format hash that embeds the salt and parameters.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2().hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verifies a plaintext password against a stored PHC-format hash.
///
/// Returns `false` for any failure — malformed hashes, verification mismatch,
/// etc. — so callers can respond with a single generic error.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => argon2()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}
