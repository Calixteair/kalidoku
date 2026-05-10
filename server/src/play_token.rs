//! HMAC play token. Signs `{game_id, device_id, started_at}` so the server can verify
//! a `POST /play` belongs to the freshly minted game without round-tripping the DB.
//!
//! Format: `b64url(payload).b64url(sig)` where payload = JSON, sig = HMAC-SHA256.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine as _};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub const TTL_SECONDS: i64 = 60 * 60; // 1h

#[derive(Debug, thiserror::Error)]
pub enum PlayTokenError {
    #[error("malformed token")]
    Malformed,
    #[error("invalid signature")]
    BadSignature,
    #[error("token expired")]
    Expired,
    #[error("token does not match the request")]
    Mismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayTokenPayload {
    #[serde(rename = "g")]
    pub game_id: Uuid,
    #[serde(rename = "d")]
    pub device_id: Uuid,
    #[serde(rename = "s")]
    pub started_at: DateTime<Utc>,
    #[serde(rename = "e")]
    pub expires_at: DateTime<Utc>,
}

impl PlayTokenPayload {
    #[must_use]
    pub fn new(game_id: Uuid, device_id: Uuid, started_at: DateTime<Utc>) -> Self {
        Self {
            game_id,
            device_id,
            started_at,
            expires_at: started_at + Duration::seconds(TTL_SECONDS),
        }
    }
}

pub fn encode(payload: &PlayTokenPayload, hmac_key: &[u8]) -> Result<String, PlayTokenError> {
    let raw = serde_json::to_vec(payload).map_err(|_| PlayTokenError::Malformed)?;
    let mut mac = HmacSha256::new_from_slice(hmac_key).map_err(|_| PlayTokenError::BadSignature)?;
    mac.update(&raw);
    let sig = mac.finalize().into_bytes();
    Ok(format!("{}.{}", B64.encode(&raw), B64.encode(sig)))
}

pub fn decode(token: &str, hmac_key: &[u8]) -> Result<PlayTokenPayload, PlayTokenError> {
    let (raw_b64, sig_b64) = token.split_once('.').ok_or(PlayTokenError::Malformed)?;
    let raw = B64.decode(raw_b64).map_err(|_| PlayTokenError::Malformed)?;
    let sig = B64.decode(sig_b64).map_err(|_| PlayTokenError::Malformed)?;

    let mut mac = HmacSha256::new_from_slice(hmac_key).map_err(|_| PlayTokenError::BadSignature)?;
    mac.update(&raw);
    let expected = mac.finalize().into_bytes();
    if !constant_time_eq::constant_time_eq(&sig, expected.as_slice()) {
        return Err(PlayTokenError::BadSignature);
    }
    let payload: PlayTokenPayload =
        serde_json::from_slice(&raw).map_err(|_| PlayTokenError::Malformed)?;
    if Utc::now() > payload.expires_at {
        return Err(PlayTokenError::Expired);
    }
    Ok(payload)
}

/// Verifies that a decoded token matches the (game_id, device_id) of the current request.
pub fn assert_matches(
    payload: &PlayTokenPayload,
    expected_game: Uuid,
    expected_device: Uuid,
) -> Result<(), PlayTokenError> {
    if payload.game_id != expected_game || payload.device_id != expected_device {
        return Err(PlayTokenError::Mismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"super-secret-key-for-tests-only!";

    #[test]
    fn encode_decode_roundtrip() {
        let p = PlayTokenPayload::new(Uuid::now_v7(), Uuid::now_v7(), Utc::now());
        let tok = encode(&p, KEY).unwrap();
        let back = decode(&tok, KEY).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn wrong_key_fails() {
        let p = PlayTokenPayload::new(Uuid::now_v7(), Uuid::now_v7(), Utc::now());
        let tok = encode(&p, KEY).unwrap();
        assert!(matches!(
            decode(&tok, b"another-key-but-32-bytes-long..!").unwrap_err(),
            PlayTokenError::BadSignature
        ));
    }

    #[test]
    fn tampered_payload_fails() {
        let p = PlayTokenPayload::new(Uuid::now_v7(), Uuid::now_v7(), Utc::now());
        let tok = encode(&p, KEY).unwrap();
        let parts: Vec<&str> = tok.split('.').collect();
        let bad = format!("{}-tamper.{}", parts[0], parts[1]);
        assert!(decode(&bad, KEY).is_err());
    }

    #[test]
    fn mismatch_detected() {
        let p = PlayTokenPayload::new(Uuid::now_v7(), Uuid::now_v7(), Utc::now());
        let other = Uuid::now_v7();
        assert!(assert_matches(&p, other, p.device_id).is_err());
    }
}
