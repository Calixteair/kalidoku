//! Insert generated grids into Postgres.
//!
//! Schema (owned by agent C, see `docs/agents/agent-c-server.md` §1) :
//! ```sql
//! grids (id uuid pk, domain text fk, mode text, publish_at timestamptz,
//!        payload jsonb, seed bigint, UNIQUE (domain, mode, publish_at))
//! ```
//! We use `INSERT ... ON CONFLICT DO NOTHING` to stay idempotent on retries.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use uuid::Uuid;

const INSERT_SQL: &str = "INSERT INTO grids (id, domain, mode, publish_at, payload, seed) \
     VALUES ($1, $2, $3, $4, $5, $6) \
     ON CONFLICT (domain, mode, publish_at) DO NOTHING";

#[derive(Debug)]
pub struct GridRecord<'a> {
    pub domain_id: &'a str,
    pub mode: &'a str,
    pub publish_at: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub seed: i64,
}

/// Returns true if a new row was actually inserted, false if `ON CONFLICT`
/// suppressed the write (i.e. a concurrent worker beat us to it).
pub async fn insert_grid(conn: &DatabaseConnection, rec: &GridRecord<'_>) -> Result<bool> {
    let id = Uuid::now_v7();
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        INSERT_SQL,
        [
            id.into(),
            rec.domain_id.into(),
            rec.mode.into(),
            rec.publish_at.into(),
            rec.payload.clone().into(),
            rec.seed.into(),
        ],
    );
    let res = conn.execute(stmt).await.context("inserting grid row")?;
    Ok(res.rows_affected() == 1)
}
