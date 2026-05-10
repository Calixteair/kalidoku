//! kalidoku-worker — cron + on-demand grid generation.
//!
//! Modes:
//! - `--cron`    : start the long-running scheduler (daily generation 00:01 UTC).
//! - `--once`    : run a single generation pass for every active domain (CI / smoke tests).
//! - `--queue`   : process the Redis solo queue (BLPOP `solo:queue`).
//! - `--reindex` : push every `domains/*/entities.json` to Meilisearch.

use anyhow::Result;
use clap::Parser;

use kalidoku_worker::cli::{Cli, Mode};
use kalidoku_worker::{cron, once, queue, reindex, telemetry};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    telemetry::init();

    let cli = Cli::parse();
    match cli.mode() {
        Mode::Cron => cron::run().await,
        Mode::Once { domain } => once::run(domain).await,
        Mode::Queue => queue::run().await,
        Mode::Reindex => reindex::run().await,
    }
}
