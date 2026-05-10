//! Typed config loader. Reads from env (populated by bao-agent on prod, .env on dev).
//! Never call `std::env::var` directly elsewhere — clippy enforces this via clippy.toml.

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub session_hmac_key: String,
    pub play_token_hmac_key: String,

    pub keycloak_issuer_url: String,
    pub keycloak_client_id: String,
    pub keycloak_client_secret: String,
    pub keycloak_redirect_url: String,

    /// HMAC-SHA256 key used to sign Altcha challenges and verify their solutions.
    /// Provisioned by bao-agent from `secret/kalidoku/prod/anti-bot.ALTCHA_HMAC_KEY`.
    #[serde(default)]
    pub altcha_hmac_key: String,

    #[serde(default = "default_log_level")]
    pub rust_log: String,

    #[serde(default)]
    pub allowed_origins: String,
}

fn default_log_level() -> String {
    "info,kalidoku=debug,tower_http=info".into()
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
