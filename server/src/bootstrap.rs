//! One-shot fixups run after migrations. Keeps the runtime DB in sync with
//! the code's view of the world (active domains, etc.).

use anyhow::{Context, Result};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter,
};

use crate::entities::domains;
use crate::state::DomainSummary;

/// Upsert one row in the `domains` table per active DomainSummary.
///
/// The `grids.domain` foreign key requires a row in `domains` before any
/// generator can run, so without this bootstrap the very first solo or
/// daily on a new domain 500s with a `grids_domain_fkey` violation. We do
/// it on every boot rather than in a migration so the migration history
/// stays free of data + code remains the single source of truth for the
/// active list.
pub async fn upsert_active_domains(
    db: &DatabaseConnection,
    active: &[DomainSummary],
) -> Result<()> {
    for d in active {
        upsert_one(db, d).await?;
    }
    Ok(())
}

async fn upsert_one(db: &DatabaseConnection, d: &DomainSummary) -> Result<()> {
    let existing = domains::Entity::find()
        .filter(domains::Column::Id.eq(d.id.clone()))
        .one(db)
        .await
        .with_context(|| format!("lookup domain '{}'", d.id))?;
    match existing {
        Some(row) => {
            let mut active_model = row.into_active_model();
            active_model.version = Set(d.version.clone());
            active_model.active = Set(true);
            active_model
                .update(db)
                .await
                .with_context(|| format!("update domain '{}'", d.id))?;
        }
        None => {
            let active_model = domains::ActiveModel {
                id: Set(d.id.clone()),
                version: Set(d.version.clone()),
                active: Set(true),
                metadata: Set(serde_json::json!({})),
                created_at: Set(Utc::now().into()),
            };
            active_model
                .insert(db)
                .await
                .with_context(|| format!("insert domain '{}'", d.id))?;
        }
    }
    Ok(())
}
