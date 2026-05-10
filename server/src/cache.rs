//! Valkey/Redis pool. Used for rate-limit, altcha replay-protection, ephemeral state.

use anyhow::{Context, Result};
use deadpool_redis::{Config, Pool, Runtime};

/// Build the application's Redis pool.
pub fn connect(redis_url: &str) -> Result<Pool> {
    let cfg = Config::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .context("building Redis pool")
}
