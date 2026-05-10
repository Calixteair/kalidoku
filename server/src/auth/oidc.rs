//! OIDC PKCE state. The actual code-exchange + JWKS handshake lives in `routes/auth.rs`,
//! this module owns the typed state struct and the PKCE helpers so that:
//!  - the route handler stays slim,
//!  - we can unit-test PKCE generation without a network.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

/// Generate a fresh PKCE pair. The verifier is 43..128 chars, S256 challenge.
#[must_use]
pub fn generate_pkce() -> PkcePair {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    let verifier = B64.encode(buf);
    let challenge = B64.encode(Sha256::digest(verifier.as_bytes()));
    PkcePair {
        verifier,
        challenge,
    }
}

#[derive(Debug, Clone)]
pub struct OidcState {
    pub state: String,
    pub nonce: String,
    pub pkce: PkcePair,
    pub redirect_to: String,
}

impl OidcState {
    #[must_use]
    pub fn fresh(redirect_to: impl Into<String>) -> Self {
        let mut state_buf = [0u8; 16];
        let mut nonce_buf = [0u8; 16];
        OsRng.fill_bytes(&mut state_buf);
        OsRng.fill_bytes(&mut nonce_buf);
        Self {
            state: B64.encode(state_buf),
            nonce: B64.encode(nonce_buf),
            pkce: generate_pkce(),
            redirect_to: redirect_to.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_lengths_within_spec() {
        let p = generate_pkce();
        assert!(p.verifier.len() >= 43);
        assert!(p.verifier.len() <= 128);
        assert!(!p.challenge.is_empty());
    }
}
