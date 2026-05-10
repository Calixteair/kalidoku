//! kalidoku-worker — cron + on-demand grid generation. Owned by agent B.
//!
//! Modes:
//! - `--cron`  : start the long-running scheduler (daily generation 00:01 UTC).
//! - `--once`  : run a single generation pass (used in CI / smoke tests).

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::warn!(
        "agent-B: implement cron + on-demand generation (see docs/agents/agent-b-worker.md)"
    );
    Ok(())
}
