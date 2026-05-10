//! Altcha (self-hosted) proof-of-work challenge issuance and verification.
//!
//! See `docs/altcha-setup.md`. The replay-check (Redis SETNX) lives in the calling
//! service so this module stays pure & easy to unit-test.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub const DEFAULT_MAX_NUMBER: u64 = 100_000;

#[derive(Debug, thiserror::Error)]
pub enum AltchaError {
    #[error("invalid HMAC key")]
    InvalidKey,
    #[error("malformed solution payload")]
    Malformed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Challenge {
    pub algorithm: &'static str,
    pub challenge: String,
    pub salt: String,
    pub signature: String,
    pub maxnumber: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Solution {
    pub algorithm: String,
    pub challenge: String,
    pub salt: String,
    pub signature: String,
    pub number: u64,
}

/// Build a fresh challenge. The secret integer is uniformly random in `[0, maxnumber)`.
pub fn issue_challenge(hmac_key: &[u8], maxnumber: u64) -> Result<Challenge, AltchaError> {
    if hmac_key.is_empty() {
        return Err(AltchaError::InvalidKey);
    }
    let mut salt_bytes = [0u8; 16];
    OsRng.fill_bytes(&mut salt_bytes);
    let salt = hex::encode(salt_bytes);

    let max = maxnumber.max(1);
    let secret_number: u64 = secure_random_u64() % max;

    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(secret_number.to_string().as_bytes());
    let challenge = hex::encode(hasher.finalize());

    let mut mac = HmacSha256::new_from_slice(hmac_key).map_err(|_| AltchaError::InvalidKey)?;
    mac.update(challenge.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    Ok(Challenge {
        algorithm: "SHA-256",
        challenge,
        salt,
        signature,
        maxnumber: max,
    })
}

/// Decode a base64-encoded JSON solution sent by the front Altcha widget.
pub fn decode_solution(b64_payload: &str) -> Result<Solution, AltchaError> {
    let bytes = B64
        .decode(b64_payload.as_bytes())
        .map_err(|_| AltchaError::Malformed)?;
    serde_json::from_slice(&bytes).map_err(|_| AltchaError::Malformed)
}

/// Constant-time verification of the signature & re-computation of the SHA-256 step.
#[must_use]
pub fn verify_solution(solution: &Solution, hmac_key: &[u8]) -> bool {
    if solution.algorithm != "SHA-256" {
        return false;
    }
    let Ok(mut mac) = HmacSha256::new_from_slice(hmac_key) else {
        return false;
    };
    mac.update(solution.challenge.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    if !constant_time_eq::constant_time_eq(expected.as_bytes(), solution.signature.as_bytes()) {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(solution.salt.as_bytes());
    hasher.update(solution.number.to_string().as_bytes());
    let computed = hex::encode(hasher.finalize());
    constant_time_eq::constant_time_eq(computed.as_bytes(), solution.challenge.as_bytes())
}

fn secure_random_u64() -> u64 {
    let mut buf = [0u8; 8];
    OsRng.fill_bytes(&mut buf);
    u64::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"test-key-32-bytes-long-padding!!";

    #[test]
    fn solving_the_challenge_makes_verify_succeed() {
        let ch = issue_challenge(KEY, 1_000).unwrap();
        // brute-force the number (bounded by maxnumber).
        let mut found: Option<u64> = None;
        for n in 0..ch.maxnumber {
            let mut h = Sha256::new();
            h.update(ch.salt.as_bytes());
            h.update(n.to_string().as_bytes());
            if hex::encode(h.finalize()) == ch.challenge {
                found = Some(n);
                break;
            }
        }
        let n = found.expect("brute force should always find the number");
        let sol = Solution {
            algorithm: "SHA-256".into(),
            challenge: ch.challenge.clone(),
            salt: ch.salt.clone(),
            signature: ch.signature.clone(),
            number: n,
        };
        assert!(verify_solution(&sol, KEY));
    }

    #[test]
    fn wrong_number_is_rejected() {
        let ch = issue_challenge(KEY, 1_000).unwrap();
        let sol = Solution {
            algorithm: "SHA-256".into(),
            challenge: ch.challenge,
            salt: ch.salt,
            signature: ch.signature,
            number: 999_999_999,
        };
        assert!(!verify_solution(&sol, KEY));
    }

    #[test]
    fn tampered_signature_is_rejected() {
        let ch = issue_challenge(KEY, 1_000).unwrap();
        let bad_sig = "00".repeat(32);
        let sol = Solution {
            algorithm: "SHA-256".into(),
            challenge: ch.challenge,
            salt: ch.salt,
            signature: bad_sig,
            number: 0,
        };
        assert!(!verify_solution(&sol, KEY));
    }

    #[test]
    fn empty_key_is_rejected() {
        let err = issue_challenge(&[], 100).unwrap_err();
        matches!(err, AltchaError::InvalidKey);
    }
}
