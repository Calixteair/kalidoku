//! Shared application state. Cloneable (everything inside is `Arc`).

use std::collections::HashMap;
use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;

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
}

impl AppState {
    #[must_use]
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(config),
            db: None,
            redis: None,
            domains: Arc::new(HashMap::new()),
        }
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
}
