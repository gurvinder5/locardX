use chrono::Utc;
use rand::RngCore;

/// Default session validity period (8 hours in seconds).
pub const SESSION_TTL_SECONDS: i64 = 8 * 3600;

/// Generates a cryptographically random, hex-encoded 256-bit session token.
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Computes the absolute expiration timestamp in UTC seconds.
pub fn calculate_expiration() -> i64 {
    Utc::now().timestamp() + SESSION_TTL_SECONDS
}

/// Checks if a session timestamp has expired.
pub fn is_expired(expires_at: i64) -> bool {
    Utc::now().timestamp() >= expires_at
}
