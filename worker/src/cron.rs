//! `--cron` mode : long-running scheduler.
//!
//! - Daily job at `00:01 UTC` (cron expression `0 1 0 * * *`) that triggers
//!   the same logic as `--once`.
//! - Healthcheck loop that logs `alive` every 5 minutes.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::once;

const DAILY_CRON_EXPR: &str = "0 1 0 * * *"; // sec min hour dom month dow — 00:01:00 UTC.
const HEALTHCHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);

pub async fn run() -> Result<()> {
    tracing::info!("kalidoku-worker --cron starting");

    let scheduler = JobScheduler::new()
        .await
        .context("creating cron scheduler")?;

    let job = Job::new_async(DAILY_CRON_EXPR, |_uuid, _l| {
        Box::pin(async move {
            tracing::info!("daily cron tick — running --once pass");
            if let Err(e) = once::run(None).await {
                tracing::error!(error = %e, "daily generation pass failed");
            }
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

async fn healthcheck_loop() {
    let mut ticker = tokio::time::interval(HEALTHCHECK_INTERVAL);
    // Skip the immediate first tick so the very first log isn't `alive` at t=0.
    ticker.tick().await;
    loop {
        ticker.tick().await;
        tracing::info!("alive");
    }
}
