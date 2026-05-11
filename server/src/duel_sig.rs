//! HMAC signature for duel share links.
//!
//! A duel URL looks like `/duel/{duel_id}?sig={b64url(hmac)}`. The signature
//! binds (duel_id, grid_id) so a leaked link can't be forged to point at a
//! different grid (e.g. tomorrow's daily) by hand-editing the URL.
//!
//! Stored verbatim in `duels.share_sig` so verification doesn't recompute on
//! every GET — we just constant-time compare.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error)]
pub enum DuelSigError {
    #[error("malformed signature")]
    Malformed,
    #[error("invalid signature")]
    BadSignature,
}

/// Compute the raw HMAC bytes for a (duel_id, grid_id) pair.
pub fn sign(key: &[u8], duel_id: Uuid, grid_id: Uuid) -> Result<Vec<u8>, DuelSigError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| DuelSigError::BadSignature)?;
    mac.update(duel_id.as_bytes());
    mac.update(grid_id.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Encode signature bytes to the URL-safe base64 form sent to the client.
#[must_use]
pub fn encode(sig: &[u8]) -> String {
    B64.encode(sig)
}

/// Decode the URL-safe base64 form from a `?sig=` query parameter.
pub fn decode(sig: &str) -> Result<Vec<u8>, DuelSigError> {
    B64.decode(sig).map_err(|_| DuelSigError::Malformed)
}

/// Constant-time verify that `provided` matches HMAC(key, duel_id || grid_id).
pub fn verify(
    key: &[u8],
    duel_id: Uuid,
    grid_id: Uuid,
    provided: &[u8],
) -> Result<(), DuelSigError> {
    let expected = sign(key, duel_id, grid_id)?;
    if !constant_time_eq::constant_time_eq(provided, &expected) {
        return Err(DuelSigError::BadSignature);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"test-duel-key-32-bytes-long-pad!";

    #[test]
    fn sign_then_verify_roundtrips() {
        let d = Uuid::now_v7();
        let g = Uuid::now_v7();
        let s = sign(KEY, d, g).unwrap();
        assert!(verify(KEY, d, g, &s).is_ok());
    }

    #[test]
    fn wrong_key_fails() {
        let d = Uuid::now_v7();
        let g = Uuid::now_v7();
        let s = sign(KEY, d, g).unwrap();
        assert!(matches!(
            verify(b"another-key-32-bytes-long-padd!!", d, g, &s).unwrap_err(),
            DuelSigError::BadSignature
        ));
    }

    #[test]
    fn swapped_grid_fails() {
        let d = Uuid::now_v7();
        let g = Uuid::now_v7();
        let g_other = Uuid::now_v7();
        let s = sign(KEY, d, g).unwrap();
        assert!(verify(KEY, d, g_other, &s).is_err());
    }

    #[test]
    fn base64_roundtrip() {
        let d = Uuid::now_v7();
        let g = Uuid::now_v7();
        let s = sign(KEY, d, g).unwrap();
        let enc = encode(&s);
        let dec = decode(&enc).unwrap();
        assert_eq!(s, dec);
    }
}
