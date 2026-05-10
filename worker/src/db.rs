//! Database connection helpers for the worker.
//!
//! We deliberately stay raw-SQL (`Statement`) rather than depend on agent C's
//! generated entities — this keeps `worker/` deployable even before the
//! `grids` migration lands, and avoids a cross-crate compile coupling.

use anyhow::{Context, Result};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr,
    Statement,
};

const TABLE_PROBE_SQL: &str = "SELECT to_regclass('public.grids') IS NOT NULL AS has_table";

/// Outcome of `connect`: either a usable pool, or a documented degraded state.
pub enum DbState {
    /// `DATABASE_URL` was set, the connection is up and the `grids` table exists.
    Ready(DatabaseConnection),
    /// Connection works but `grids` table is missing (agent C migrations pending).
    /// Caller should log + exit 0.
    SchemaMissing,
    /// `DATABASE_URL` not set. Caller should log + exit 0 in CI / dev.
    NoDatabaseUrl,
    /// Connection / probe failed. Caller decides whether to bail or continue.
    Unreachable(String),
}

#[allow(clippy::disallowed_methods)] // worker has no central config service yet.
pub async fn connect() -> Result<DbState> {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return Ok(DbState::NoDatabaseUrl);
    };
    let mut opts = ConnectOptions::new(url);
    opts.sqlx_logging(false);

    let conn = match Database::connect(opts).await {
        Ok(c) => c,
        Err(e) => return Ok(DbState::Unreachable(e.to_string())),
    };

    match grids_table_exists(&conn).await {
        Ok(true) => Ok(DbState::Ready(conn)),
        Ok(false) => Ok(DbState::SchemaMissing),
        Err(e) => Ok(DbState::Unreachable(e.to_string())),
    }
}

async fn grids_table_exists(conn: &DatabaseConnection) -> Result<bool, DbErr> {
    let stmt = Statement::from_string(DatabaseBackend::Postgres, TABLE_PROBE_SQL.to_owned());
    let row = conn
        .query_one(stmt)
        .await?
        .ok_or_else(|| DbErr::Custom("empty result for table probe".into()))?;
    row.try_get::<bool>("", "has_table")
}

/// True iff a row with `(domain, mode='daily', publish_at::date = $2)` exists.
pub async fn has_daily_grid(
    conn: &DatabaseConnection,
    domain_id: &str,
    iso_date: &str,
) -> Result<bool> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT EXISTS (\
            SELECT 1 FROM grids \
            WHERE domain = $1 AND mode = 'daily' AND publish_at::date = $2::date\
         ) AS hit",
        [domain_id.into(), iso_date.into()],
    );
    let row = conn
        .query_one(stmt)
        .await
        .context("probing existing daily grid")?
        .ok_or_else(|| anyhow::anyhow!("no row returned by EXISTS query"))?;
    let hit: bool = row.try_get("", "hit")?;
    Ok(hit)
}
