use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use locardx_common::LocardError;

/// Hashes a plaintext password using Argon2id with a cryptographically secure random salt.
/// SECURITY INVARIANT: Passwords and generated hashes MUST NEVER be logged.
pub fn hash_password(password: &str) -> Result<String, LocardError> {
    validate_password_strength(password)?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| LocardError::Internal(format!("Password hashing failed: {}", e)))?
        .to_string();

    Ok(password_hash)
}

/// Verifies a plaintext password against a stored PHC-format Argon2 hash.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, LocardError> {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => return Ok(false),
    };

    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Validates that a password satisfies minimum security complexity standards.
/// - At least 10 characters long
/// - Contains at least one uppercase letter
/// - Contains at least one lowercase letter
/// - Contains at least one numeric digit
/// - Contains at least one special/punctuation character
pub fn validate_password_strength(password: &str) -> Result<(), LocardError> {
    if password.len() < 10 {
        return Err(LocardError::SecurityViolation(
            "Password must be at least 10 characters long.".to_string(),
        ));
    }

    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_uppercase || !has_lowercase || !has_digit || !has_special {
        return Err(LocardError::SecurityViolation(
            "Password must contain uppercase, lowercase, digit, and special characters."
                .to_string(),
        ));
    }

    Ok(())
}
