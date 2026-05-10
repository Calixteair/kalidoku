//! `--cron` mode : long-running scheduler.
//!
//! - Daily job at `00:01 UTC` (cron expression `0 1 0 * * *`) that triggers
//!   the same logic as `--once`, then re-pushes every domain pack to
//!   Meilisearch so newly generated daily payloads share the same fresh
//!   autocomplete index.
//! - One-shot Meilisearch reindex at boot so a freshly-deployed worker
//!   primes the search index before the first daily tick.
//! - Healthcheck loop that logs `alive` every 5 minutes.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{domain_pack, once, reindex};

const DAILY_CRON_EXPR: &str = "0 1 0 * * *"; // sec min hour dom month dow — 00:01:00 UTC.
const HEALTHCHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);

pub async fn run() -> Result<()> {
    tracing::info!("kalidoku-worker --cron starting");

    // Best-effort initial reindex. We log warn on missing env vars so a fresh
    // dev deploy without Meili does not panic the whole worker.
    reindex_best_effort().await;

    let scheduler = JobScheduler::new()
        .await
        .context("creating cron scheduler")?;

    let job = Job::new_async(DAILY_CRON_EXPR, |_uuid, _l| {
        Box::pin(async move {
            tracing::info!("daily cron tick — running --once pass");
            if let Err(e) = once::run(None).await {
                tracing::error!(error = %e, "daily generation pass failed");
            }
            // Reindex right after generation so the autocomplete reflects
            // any entity refresh that landed with the new daily grid.
            reindex_best_effort().await;
        })
    })
    .context("building daily cron job")?;

    scheduler
        .add(job)
        .await
        .context("registering daily cron job")?;

    scheduler.start().await.context("starting scheduler")?;
    tracing::info!(cron = %DAILY_CRON_EXPR, "scheduler running");

    healthcheck_loop().await;

    Ok(())
}

/// Trigger `reindex_all` if `MEILI_URL` + `MEILI_MASTER_KEY` are set.
/// Errors are logged but never propagated : reindex is idempotent and can
/// always be retried by the next cron tick or by an operator running
/// `--reindex` manually.
#[allow(clippy::disallowed_methods)] // worker has no central config service yet.
async fn reindex_best_effort() {
    let url = std::env::var("MEILI_URL").ok();
    let key = std::env::var("MEILI_MASTER_KEY").ok();
    let (Some(url), Some(key)) = (url, key) else {
        tracing::warn!("MEILI_URL / MEILI_MASTER_KEY not set, skipping meilisearch reindex");
        return;
    };
    let domains_dir = domain_pack::default_root();
    if let Err(err) = reindex::reindex_all(&url, &key, &domains_dir).await {
        tracing::error!(error = %err, "meilisearch reindex failed");
    }
}

async fn healthcheck_loop() {
    let mut ticker = tokio::time::interval(HEALTHCHECK_INTERVAL);
    // Skip the immediate first tick so the very first log isn't `alive` at t=0.
    ticker.tick().await;
    loop {
        ticker.tick().await;
        tracing::info!("alive");
    }
}
