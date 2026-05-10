//! Minimal JWKS fetcher with TTL cache. We hit the realm's `/protocol/openid-connect/certs`
//! endpoint at most once per `CACHE_TTL`, then validate id_tokens against the cached set.
//!
//! Production-grade rotation/forced-refresh on a kid miss is a follow-up — for the MVP we
//! refetch on cache miss, which is enough as long as the realm doesn't rotate keys mid-day.

use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

use super::jwt::{IdTokenClaims, JwtError};

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Clone)]
struct CachedKey {
    decoding: Arc<DecodingKey>,
    alg: Algorithm,
}

#[derive(Default)]
struct Cache {
    keys: std::collections::HashMap<String, CachedKey>,
    fetched_at: Option<Instant>,
}

#[derive(Clone)]
pub struct Jwks {
    issuer: String,
    audience: String,
    jwks_uri: String,
    http: reqwest::Client,
    cache: Arc<RwLock<Cache>>,
}

impl Jwks {
    /// Build a JWKS validator pointing at `<issuer>/protocol/openid-connect/certs`.
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        let issuer = issuer.into();
        let jwks_uri = format!(
            "{}/protocol/openid-connect/certs",
            issuer.trim_end_matches('/')
        );
        Self {
            issuer,
            audience: audience.into(),
            jwks_uri,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("static reqwest client"),
            cache: Arc::new(RwLock::new(Cache::default())),
        }
    }

    /// Validate a Keycloak id_token. Fetches the JWKS on cache miss / TTL expiry.
    pub async fn validate(&self, raw: &str) -> Result<IdTokenClaims, JwtError> {
        let header = decode_header(raw).map_err(|_| JwtError::BadHeader)?;
        let kid = header.kid.ok_or(JwtError::BadHeader)?;

        if let Some(cached) = self.cache_lookup(&kid).await {
            return self.validate_with(raw, &cached);
        }
        self.refresh().await?;
        let cached = self
            .cache_lookup(&kid)
            .await
            .ok_or_else(|| JwtError::Validation(format!("kid {kid} not in JWKS")))?;
        self.validate_with(raw, &cached)
    }

    async fn cache_lookup(&self, kid: &str) -> Option<CachedKey> {
        let cache = self.cache.read().await;
        let stale = cache
            .fetched_at
            .is_none_or(|t| t.elapsed() >= CACHE_TTL);
        if stale {
            return None;
        }
        cache.keys.get(kid).cloned()
    }

    async fn refresh(&self) -> Result<(), JwtError> {
        let body: JwkSet = self
            .http
            .get(&self.jwks_uri)
            .send()
            .await
            .map_err(|e| JwtError::Validation(format!("JWKS fetch: {e}")))?
            .json()
            .await
            .map_err(|e| JwtError::Validation(format!("JWKS parse: {e}")))?;

        let mut new_cache = Cache {
            keys: std::collections::HashMap::new(),
            fetched_at: Some(Instant::now()),
        };
        for jwk in body.keys {
            if jwk.kty != "RSA" {
                continue;
            }
            let (Some(n), Some(e)) = (jwk.n, jwk.e) else {
                continue;
            };
            let alg = match jwk.alg.as_deref() {
                Some("RS256") | None => Algorithm::RS256,
                Some("RS384") => Algorithm::RS384,
                Some("RS512") => Algorithm::RS512,
                _ => continue,
            };
            let Ok(decoding) = DecodingKey::from_rsa_components(&n, &e) else {
                continue;
            };
            new_cache.keys.insert(
                jwk.kid,
                CachedKey {
                    decoding: Arc::new(decoding),
                    alg,
                },
            );
        }

        let mut writer = self.cache.write().await;
        *writer = new_cache;
        Ok(())
    }

    fn validate_with(&self, raw: &str, key: &CachedKey) -> Result<IdTokenClaims, JwtError> {
        let mut validation = Validation::new(key.alg);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        let data = decode::<IdTokenClaims>(raw, &key.decoding, &validation)
            .map_err(|e| JwtError::Validation(e.to_string()))?;
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwks_uri_is_well_formed() {
        let j = Jwks::new("https://auth.calixteair.fr/realms/kalidoku", "kalidoku-web");
        assert_eq!(
            j.jwks_uri,
            "https://auth.calixteair.fr/realms/kalidoku/protocol/openid-connect/certs"
        );
    }

    #[test]
    fn jwks_uri_handles_trailing_slash() {
        let j = Jwks::new(
            "https://auth.calixteair.fr/realms/kalidoku/",
            "kalidoku-web",
        );
        assert_eq!(
            j.jwks_uri,
            "https://auth.calixteair.fr/realms/kalidoku/protocol/openid-connect/certs"
        );
    }
}
