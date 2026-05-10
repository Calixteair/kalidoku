//! kalidoku-server entry point. Owned by agent C.
//!
//! Wires:
//! - tracing (JSON logs to stdout, captured by Wazuh agent)
//! - config (loaded from env, populated by bao-agent at /run/kalidoku/.env)
//! - DB pool (SeaORM)
//! - Redis pool (Valkey)
//! - HTTP router (see modules `routes`, `auth`, `play`, `leaderboard`, `domains`)
//! - tower middlewares (governor, cors, set-header, timeout, trace)

use std::net::SocketAddr;

use anyhow::Result;

mod config;
mod telemetry;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init();
    tracing::info!("kalidoku-server booting");

    let cfg = config::load()?;
    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.port).parse()?;

    tracing::warn!(
        addr = %addr,
        "agent-C: implement router, DB, cache, auth (see docs/agents/agent-c-server.md)"
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let app = axum::Router::new().route("/api/health", axum::routing::get(health));
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "time": chrono::Utc::now().to_rfc3339(),
    }))
}
