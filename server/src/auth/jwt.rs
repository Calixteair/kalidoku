//! Lightweight JWT validator. The MVP only needs to read claims off a Keycloak-issued
//! id_token after the OIDC code exchange — full JWKS rotation can come later.

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    pub exp: i64,
    pub iat: i64,
    pub iss: String,
    #[serde(default)]
    pub aud: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("malformed JWT header")]
    BadHeader,
    #[error("validation failed: {0}")]
    Validation(String),
}

/// Minimal id_token decode used by the OIDC callback. It checks signature, issuer
/// and `aud` claim. Production hardening (JWKS fetch + cache + rotation) is a TODO
/// for the auth hardening pass.
pub fn decode_id_token(
    raw: &str,
    issuer: &str,
    audience: &str,
    public_key_pem: &[u8],
) -> Result<IdTokenClaims, JwtError> {
    let header = decode_header(raw).map_err(|_| JwtError::BadHeader)?;
    let alg = match header.alg {
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => header.alg,
        _ => return Err(JwtError::BadHeader),
    };
    let mut validation = Validation::new(alg);
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[audience]);
    let key = DecodingKey::from_rsa_pem(public_key_pem)
        .map_err(|e| JwtError::Validation(e.to_string()))?;
    let data = decode::<IdTokenClaims>(raw, &key, &validation)
        .map_err(|e| JwtError::Validation(e.to_string()))?;
    Ok(data.claims)
}
