//! kalidoku-server entry point. Owned by agent C.
//!
//! Wires:
//! - tracing (JSON logs to stdout)
//! - config (loaded from env, populated by bao-agent at /run/kalidoku/.env)
//! - DB pool (SeaORM)
//! - Redis pool (Valkey)
//! - HTTP router (see crate::build_router)

#![forbid(unsafe_code)]

use std::net::SocketAddr;

use anyhow::Result;
use kalidoku_server::{build_router, cache, config, db, state::AppState, telemetry};
use sea_orm_migration::MigratorTrait;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init();

    let cfg = config::load()?;
    info!(port = cfg.port, "kalidoku-server booting");

    let mut state = AppState::new(cfg.clone());

    if !cfg.database_url.is_empty() {
        match db::connect(&cfg.database_url).await {
            Ok(conn) => {
                if cfg.run_migrations {
                    kalidoku_server::migrations::Migrator::up(&conn, None).await?;
                    info!("migrations applied");
                }
                state = state.with_db(conn);
            }
            Err(e) => tracing::warn!(error = %e, "database unavailable, running degraded"),
        }
    }

    if !cfg.redis_url.is_empty() {
        match cache::connect(&cfg.redis_url) {
            Ok(pool) => state = state.with_redis(pool),
            Err(e) => tracing::warn!(error = %e, "redis unavailable, running degraded"),
        }
    }

    let app = build_router(state);
    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.port).parse()?;
    info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
