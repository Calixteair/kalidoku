//! kalidoku-worker — cron + on-demand grid generation.
//!
//! Modes:
//! - `--cron`  : start the long-running scheduler (daily generation 00:01 UTC).
//! - `--once`  : run a single generation pass for every active domain (CI / smoke tests).
//! - `--queue` : process the Redis solo queue (BLPOP `solo:queue`).

use anyhow::Result;
use clap::Parser;

mod cli;
mod cron;
mod db;
mod domain_pack;
mod once;
mod persist;
mod queue;
mod seed;
mod telemetry;

use cli::{Cli, Mode};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    telemetry::init();

    let cli = Cli::parse();
    match cli.mode() {
        Mode::Cron => cron::run().await,
        Mode::Once { domain } => once::run(domain).await,
        Mode::Queue => queue::run().await,
    }
}
