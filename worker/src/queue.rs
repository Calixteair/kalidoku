//! `--queue` mode (phase MVP+) : Redis-backed solo grid generator.
//!
//! Wire frame :
//! 1. Connect to Redis (Valkey) via `REDIS_URL`.
//! 2. `BLPOP solo:queue 0` to get a `{ "id": "<uuid>", "domain": "<id>" }` job.
//! 3. Generate the grid (same path as `--once` but mode = `solo`).
//! 4. `RPUSH solo:done:<id> <serialized_grid>` so the server can `BRPOP` it.
//!
//! The implementation is intentionally a stub today — agent C is still
//! shaping the queue payload and the server side hasn't published the keys
//! schema yet. We log, sleep, log: the binary stays alive without crashing
//! so docker-compose health-checks pass during integration.

use std::time::Duration;

use anyhow::Result;

const TODO_LOG_INTERVAL: Duration = Duration::from_secs(60);

pub async fn run() -> Result<()> {
    tracing::warn!(
        // TODO: implement BLPOP solo:queue + push solo:done:<id> once agent C
        // freezes the queue payload schema (see docs/agents/agent-c-server.md).
        queue = "solo:queue",
        result_key = "solo:done:<id>",
        "kalidoku-worker --queue is a stub — Redis BLPOP wiring pending"
    );

    let mut ticker = tokio::time::interval(TODO_LOG_INTERVAL);
    loop {
        ticker.tick().await;
        tracing::info!("queue worker idle (stub)");
    }
}
