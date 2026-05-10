//! Insert generated grids into Postgres.
//!
//! Schema (owned by agent C, see `docs/agents/agent-c-server.md` §1) :
//! ```sql
//! domains (id text pk, version text, active bool, metadata jsonb, created_at timestamptz)
//! grids   (id uuid pk, domain text fk → domains.id, mode text,
//!          publish_at timestamptz, payload jsonb, seed bigint,
//!          created_at timestamptz, UNIQUE (domain, mode, publish_at))
//! ```
//! We use `INSERT ... ON CONFLICT DO NOTHING` on `grids` to stay idempotent
//! on retries, and a small `INSERT ... ON CONFLICT DO UPDATE` on `domains` so
//! the FK is always satisfied when the worker boots before the server has
//! seeded its catalogue.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use uuid::Uuid;

const INSERT_GRID_SQL: &str =
    "INSERT INTO grids (id, domain, mode, publish_at, payload, seed, created_at) \
     VALUES ($1, $2, $3, $4, $5, $6, $7) \
     ON CONFLICT (domain, mode, publish_at) DO NOTHING";

const UPSERT_DOMAIN_SQL: &str = "INSERT INTO domains (id, version, active, metadata, created_at) \
     VALUES ($1, $2, true, $3, $4) \
     ON CONFLICT (id) DO UPDATE SET \
        version  = EXCLUDED.version, \
        active   = EXCLUDED.active, \
        metadata = EXCLUDED.metadata";

#[derive(Debug)]
pub struct GridRecord<'a> {
    pub domain_id: &'a str,
    pub mode: &'a str,
    pub publish_at: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub seed: i64,
}

#[derive(Debug)]
pub struct DomainRecord<'a> {
    pub id: &'a str,
    pub version: &'a str,
    pub metadata: serde_json::Value,
}

/// Idempotent upsert of a domain row. The FK on `grids.domain` is `RESTRICT`,
/// so we must guarantee the parent row exists before inserting any grid.
pub async fn upsert_domain(conn: &DatabaseConnection, rec: &DomainRecord<'_>) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        UPSERT_DOMAIN_SQL,
        [
            rec.id.into(),
            rec.version.into(),
            rec.metadata.clone().into(),
            Utc::now().into(),
        ],
    );
    conn.execute(stmt)
        .await
        .with_context(|| format!("upserting domain row '{}'", rec.id))?;
    Ok(())
}

/// Returns true if a new row was actually inserted, false if `ON CONFLICT`
/// suppressed the write (i.e. a concurrent worker beat us to it).
pub async fn insert_grid(conn: &DatabaseConnection, rec: &GridRecord<'_>) -> Result<bool> {
    let id = Uuid::now_v7();
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        INSERT_GRID_SQL,
        [
            id.into(),
            rec.domain_id.into(),
            rec.mode.into(),
            rec.publish_at.into(),
            rec.payload.clone().into(),
            rec.seed.into(),
            Utc::now().into(),
        ],
    );
    let res = conn.execute(stmt).await.context("inserting grid row")?;
    Ok(res.rows_affected() == 1)
}
