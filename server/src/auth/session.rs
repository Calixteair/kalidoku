//! Opaque session tokens stored as SHA-256 hash in DB, exposed to the client through
//! the `__Host-session` cookie. Devices get a long-lived `kd_device` cookie used
//! by anonymous play.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const COOKIE_SESSION: &str = "__Host-session";
pub const COOKIE_DEVICE: &str = "kd_device";
pub const SESSION_TTL_DAYS: i64 = 30;
pub const DEVICE_COOKIE_TTL_DAYS: i64 = 365;

/// 32 random bytes, base64url(no-padding) encoded. The clear value is returned to the
/// caller so it can be set on the response cookie; only the SHA-256 hash is persisted.
#[must_use]
pub fn mint_session_token() -> (String, Vec<u8>) {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    let token = B64.encode(buf);
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    (token, hash)
}

#[must_use]
pub fn hash_session_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Resolved auth context. Populated by [`crate::middleware::session::resolve`] and
/// stuffed into the request extensions; handlers pluck it via `Extension<AuthContext>`.
#[derive(Clone, Debug, Default)]
pub struct AuthContext {
    pub user_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub is_authenticated: bool,
}

impl AuthContext {
    #[must_use]
    pub fn anonymous() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hash_is_stable_and_32_bytes() {
        let (tok, hash) = mint_session_token();
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, hash_session_token(&tok));
    }

    #[test]
    fn two_mints_differ() {
        let (a, _) = mint_session_token();
        let (b, _) = mint_session_token();
        assert_ne!(a, b);
    }
}
