//! Shared application state. Cloneable (everything inside is `Arc`).

use std::collections::HashMap;
use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sea_orm::DatabaseConnection;
use tokio::sync::RwLock;

use crate::auth::jwks::Jwks;
use crate::auth::oidc::OidcState;
use crate::config::AppConfig;
use crate::services::meili::MeiliClient;

/// Static description of a domain known to the server.
#[derive(Debug, Clone)]
pub struct DomainSummary {
    pub id: String,
    pub name_fr: String,
    pub name_en: String,
    pub version: String,
    pub description: Option<String>,
    pub available_modes: Vec<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: Option<Arc<DatabaseConnection>>,
    pub redis: Option<Arc<RedisPool>>,
    pub domains: Arc<HashMap<String, DomainSummary>>,
    /// In-memory map of pending OIDC handshakes, keyed by the random `state` parameter.
    /// Single-instance only — for multi-node, move to Redis.
    pub oidc_states: Arc<RwLock<HashMap<String, OidcState>>>,
    /// Lazy JWKS validator pointing at the Keycloak realm. None when issuer is empty (tests).
    pub jwks: Option<Arc<Jwks>>,
    /// Meilisearch client for autocomplete. `None` disables the endpoint with a
    /// clean 503 (e.g. dev without the search container, or before bao-agent
    /// has provisioned the master key).
    pub meili: Option<Arc<MeiliClient>>,
}

impl AppState {
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        let jwks = if config.keycloak_issuer_url.is_empty()
            || config.keycloak_issuer_url.contains("example.invalid")
        {
            None
        } else {
            Some(Arc::new(Jwks::new(
                config.keycloak_issuer_url.clone(),
                config.keycloak_client_id.clone(),
            )))
        };
        Self {
            config: Arc::new(config),
            db: None,
            redis: None,
            domains: Arc::new(HashMap::new()),
            oidc_states: Arc::new(RwLock::new(HashMap::new())),
            jwks,
            meili: None,
        }
    }

    #[must_use]
    pub fn with_meili(mut self, meili: Arc<MeiliClient>) -> Self {
        self.meili = Some(meili);
        self
    }

    #[must_use]
    pub fn with_db(mut self, db: DatabaseConnection) -> Self {
        self.db = Some(Arc::new(db));
        self
    }

    #[must_use]
    pub fn with_redis(mut self, redis: RedisPool) -> Self {
        self.redis = Some(Arc::new(redis));
        self
    }

    #[must_use]
    pub fn with_domains(mut self, domains: HashMap<String, DomainSummary>) -> Self {
        self.domains = Arc::new(domains);
        self
    }

    /// Build a no-DB / no-Redis AppState for unit / smoke tests.
    /// All HMAC keys are set to deterministic non-empty values so signature code paths work.
    #[cfg(test)]
    #[must_use]
    pub fn for_tests() -> Self {
        Self::new(AppConfig {
            port: 0,
            database_url: String::new(),
            redis_url: String::new(),
            session_hmac_key: "test-session-hmac-key-32-bytes!!".into(),
            play_token_hmac_key: "test-play-hmac-key-32-bytes!".into(),
            keycloak_issuer_url: "https://example.invalid/realms/test".into(),
            keycloak_client_id: "test".into(),
            keycloak_client_secret: "test".into(),
            keycloak_redirect_url: "https://example.invalid/cb".into(),
            altcha_hmac_key: "test-altcha-hmac-key-32-bytes!!".into(),
            meili_url: "http://search.invalid:7700".into(),
            meili_master_key: String::new(),
            rust_log: "info".into(),
            allowed_origins: String::new(),
            run_migrations: false,
        })
    }
}
