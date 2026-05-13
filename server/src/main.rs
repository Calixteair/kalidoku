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

use std::collections::HashMap;

use anyhow::Result;
use kalidoku_server::{
    build_router, cache, config, db,
    services::meili::MeiliClient,
    state::{AppState, DomainSummary},
    telemetry,
};
use sea_orm_migration::MigratorTrait;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init();

    let cfg = config::load()?;
    info!(port = cfg.port, "kalidoku-server booting");

    // Active domains. For now hardcoded — when several domain packs land we'll
    // query the `domains` table on boot and build this from rows.
    let mut active_domains = HashMap::new();
    active_domains.insert(
        "paris-metro".to_string(),
        DomainSummary {
            id: "paris-metro".into(),
            name_fr: "Métro de Paris".into(),
            name_en: "Paris Metro".into(),
            version: "0.4.0".into(),
            description: None,
            available_modes: vec!["daily".into(), "solo".into(), "duel".into()],
        },
    );
    active_domains.insert(
        "rer".to_string(),
        DomainSummary {
            id: "rer".into(),
            name_fr: "RER d'Île-de-France".into(),
            name_en: "Île-de-France RER".into(),
            version: "0.2.0".into(),
            description: None,
            available_modes: vec!["daily".into(), "solo".into(), "duel".into()],
        },
    );
    let mut state = AppState::new(cfg.clone()).with_domains(active_domains);

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

    // Search backend. We require the master key to be set explicitly — without
    // it the autocomplete handler short-circuits with 503 rather than blasting
    // unauthenticated requests at Meili.
    if !cfg.meili_master_key.is_empty() {
        let client = MeiliClient::new(cfg.meili_url.clone(), cfg.meili_master_key.clone());
        state = state.with_meili(client);
        info!(url = %cfg.meili_url, "meilisearch client configured");
    } else {
        tracing::warn!("MEILI_MASTER_KEY empty: autocomplete disabled (503)");
    }

    let app = build_router(state);
    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.port).parse()?;
    info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // `into_make_service_with_connect_info::<SocketAddr>()` exposes the peer IP to
    // tower-governor's PeerIpKeyExtractor — without it the rate-limit middleware can't
    // identify the client and short-circuits to 500.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
