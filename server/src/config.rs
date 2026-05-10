//! Typed config loader. Reads from env (populated by bao-agent on prod, .env on dev).
//! Never call `std::env::var` directly elsewhere — clippy enforces this via clippy.toml.

use anyhow::{Context, Result};
use serde::Deserialize;

fn default_port() -> u16 {
    8080
}

fn default_log_level() -> String {
    "info,kalidoku=debug,tower_http=info".into()
}

fn default_string() -> String {
    String::new()
}

fn default_meili_url() -> String {
    "http://search:7700".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_string")]
    pub database_url: String,
    #[serde(default = "default_string")]
    pub redis_url: String,
    #[serde(default = "default_string")]
    pub session_hmac_key: String,
    #[serde(default = "default_string")]
    pub play_token_hmac_key: String,

    #[serde(default = "default_string")]
    pub keycloak_issuer_url: String,
    #[serde(default = "default_string")]
    pub keycloak_client_id: String,
    #[serde(default = "default_string")]
    pub keycloak_client_secret: String,
    #[serde(default = "default_string")]
    pub keycloak_redirect_url: String,

    /// HMAC-SHA256 key used to sign Altcha challenges and verify their solutions.
    /// Provisioned by bao-agent from `secret/kalidoku/prod/anti-bot.ALTCHA_HMAC_KEY`.
    #[serde(default)]
    pub altcha_hmac_key: String,

    /// Base URL of the Meilisearch instance backing autocomplete (no trailing
    /// slash). Defaults to the in-cluster Docker name `http://search:7700`. In
    /// dev with no search container running we leave the master key empty,
    /// which disables the autocomplete handler with a clean 503.
    #[serde(default = "default_meili_url")]
    pub meili_url: String,
    /// Master API key for Meilisearch. Empty in dev when search isn't booted.
    /// Provisioned in prod by bao-agent from `secret/kalidoku/prod/search.MEILI_MASTER_KEY`.
    #[serde(default)]
    pub meili_master_key: String,

    #[serde(default = "default_log_level")]
    pub rust_log: String,

    #[serde(default)]
    pub allowed_origins: String,

    /// When set to `true` the server runs SeaORM migrations at boot. Off by default
    /// in dev — we rely on `sea-orm-cli migrate up` so the schema lives outside the
    /// service lifecycle. Set to `true` in container deployments.
    #[serde(default)]
    pub run_migrations: bool,
}

impl AppConfig {
    /// Test-only fixture with sane defaults so we don't have to touch env vars.
    #[must_use]
    pub fn test_fixture() -> Self {
        Self {
            port: 0,
            database_url: String::new(),
            redis_url: String::new(),
            session_hmac_key: "test-session-key-32-bytes-long..".into(),
            play_token_hmac_key: "test-play-token-key-32-bytes-..!".into(),
            keycloak_issuer_url: String::new(),
            keycloak_client_id: String::new(),
            keycloak_client_secret: String::new(),
            keycloak_redirect_url: String::new(),
            altcha_hmac_key: "test-altcha-key-32-bytes-long..!".into(),
            meili_url: "http://search.invalid:7700".into(),
            meili_master_key: String::new(),
            rust_log: "warn".into(),
            allowed_origins: String::new(),
            run_migrations: false,
        }
    }
}

#[allow(clippy::disallowed_methods)] // single legitimate use of std::env::var, behind config service
pub fn load() -> Result<AppConfig> {
    dotenvy::dotenv().ok();
    let cfg = config::Config::builder()
        .add_source(config::Environment::default().separator("__"))
        .build()
        .context("building config")?;
    cfg.try_deserialize().context("deserialising AppConfig")
}
