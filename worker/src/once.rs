//! `--once` mode : iterate over discovered domains, generate today's daily grid
//! when missing, persist it.
//!
//! Designed to remain useful even when:
//! - `DATABASE_URL` is unset (local dev) → log warn + exit 0.
//! - The `grids` table is missing (agent C migrations pending) → log warn + exit 0.
//!
//! The generator is wired against `kalidoku_core::generator::generate` and the
//! payload is the `GridSnapshot` produced by `snapshot_with_entities`, so the
//! server can resolve user-typed names without re-loading the domain pack on
//! every request.

use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use sea_orm::DatabaseConnection;

use kalidoku_core::{
    domain::{load_domain_pack, Domain},
    generator::{generate, GenerationOptions},
};

use crate::{
    db::{self, DbState},
    domain_pack::{self, DiscoveredDomain},
    persist::{self, DomainRecord, GridRecord},
    seed,
};

const DAILY_PUBLISH_HOUR_UTC: u32 = 0;
const DAILY_PUBLISH_MINUTE_UTC: u32 = 1;
const GENERATOR_MAX_ATTEMPTS: u32 = 200;

/// Outcome of a single domain × date generation attempt.
#[derive(Debug, PartialEq, Eq)]
pub enum GenerationOutcome {
    /// New row inserted into `grids`.
    Inserted,
    /// A row for `(domain, mode='daily', publish_at::date)` already existed.
    AlreadyPresent,
    /// `DATABASE_URL` not set — generation ran but no INSERT was attempted.
    DryRun,
}

pub async fn run(domain_filter: Option<String>) -> Result<()> {
    let today = Utc::now().date_naive();
    tracing::info!(date = %today, "kalidoku-worker --once starting");

    let domains = list_active_domains().await?;
    if domains.is_empty() {
        tracing::warn!("no active domain found, nothing to do");
        return Ok(());
    }

    let conn = match db::connect().await? {
        DbState::Ready(c) => Some(c),
        DbState::SchemaMissing => {
            tracing::warn!(
                "database is reachable but `grids` table is missing — agent C migrations pending, exiting cleanly"
            );
            return Ok(());
        }
        DbState::NoDatabaseUrl => {
            tracing::warn!("DATABASE_URL not set — running in dry-run mode, no persistence");
            None
        }
        DbState::Unreachable(err) => {
            tracing::warn!(error = %err, "database unreachable, exiting cleanly");
            return Ok(());
        }
    };

    let mut generated = 0_usize;
    let mut skipped = 0_usize;
    let mut failed = 0_usize;

    for d in domains {
        if let Some(filter) = domain_filter.as_deref() {
            if filter != d.id {
                continue;
            }
        }

        match generate_for_domain(&d, today, conn.as_ref()).await {
            Ok(GenerationOutcome::Inserted) => generated += 1,
            Ok(GenerationOutcome::AlreadyPresent) => skipped += 1,
            Ok(GenerationOutcome::DryRun) => generated += 1,
            Err(err) => {
                failed += 1;
                tracing::error!(domain = %d.id, error = %err, "generation failed");
            }
        }
    }

    tracing::info!(generated, skipped, failed, "kalidoku-worker --once done");
    Ok(())
}

async fn list_active_domains() -> Result<Vec<DiscoveredDomain>> {
    // For now we only support the filesystem fallback. When agent C ships the
    // `domains` table + an `active=true` flag we'll wire it in here.
    domain_pack::discover(&domain_pack::default_root())
}

/// Run the full pipeline (load pack → generate → persist) for one
/// `(domain, date)` pair. Public so integration tests can drive it directly.
pub async fn generate_for_domain(
    domain: &DiscoveredDomain,
    date: NaiveDate,
    conn: Option<&DatabaseConnection>,
) -> Result<GenerationOutcome> {
    let iso_date = date.format("%Y-%m-%d").to_string();
    let publish_at = compute_publish_at(date, &iso_date)?;

    if let Some(c) = conn {
        if db::has_daily_grid(c, &domain.id, &iso_date).await? {
            tracing::info!(domain = %domain.id, date = %iso_date, "daily grid already present, skipping");
            return Ok(GenerationOutcome::AlreadyPresent);
        }
    }

    let derived_seed = seed::derive(&domain.id, date);
    tracing::info!(
        domain = %domain.id,
        date = %iso_date,
        seed = derived_seed,
        "generating daily grid"
    );

    let DomainPayload { payload, version } =
        generate_payload(&domain.id, &domain.root, derived_seed).await?;

    let Some(c) = conn else {
        tracing::warn!(
            domain = %domain.id,
            "DATABASE_URL not set, skipping INSERT (dry-run)"
        );
        return Ok(GenerationOutcome::DryRun);
    };

    persist::upsert_domain(
        c,
        &DomainRecord {
            id: &domain.id,
            version: &version,
            metadata: serde_json::Value::Null,
        },
    )
    .await?;

    let inserted = persist::insert_grid(
        c,
        &GridRecord {
            domain_id: &domain.id,
            mode: "daily",
            publish_at,
            payload,
            seed: derived_seed as i64,
        },
    )
    .await?;
    if inserted {
        Ok(GenerationOutcome::Inserted)
    } else {
        tracing::warn!(domain = %domain.id, date = %iso_date, "ON CONFLICT DO NOTHING — concurrent insert");
        Ok(GenerationOutcome::AlreadyPresent)
    }
}

fn compute_publish_at(date: NaiveDate, iso_date: &str) -> Result<DateTime<Utc>> {
    let naive = date
        .and_hms_opt(DAILY_PUBLISH_HOUR_UTC, DAILY_PUBLISH_MINUTE_UTC, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid publish_at for {iso_date}"))?;
    Ok(Utc.from_utc_datetime(&naive))
}

/// Pair `(payload, version)` returned by the core generator.
struct DomainPayload {
    payload: serde_json::Value,
    version: String,
}

/// Bridge to `core::*`. Loads the on-disk domain pack, runs the CSP generator
/// with the deterministic seed, and serialises the resulting grid (with the
/// referenced entities baked in).
async fn generate_payload(
    domain_id: &str,
    domain_root: &Path,
    seed_value: u64,
) -> Result<DomainPayload> {
    let domain_id_owned = domain_id.to_owned();
    let domain_root_owned = domain_root.to_path_buf();

    tokio::task::spawn_blocking(move || {
        generate_via_core(&domain_id_owned, &domain_root_owned, seed_value)
    })
    .await
    .context("blocking generator task crashed")?
}

fn generate_via_core(
    domain_id: &str,
    domain_root: &Path,
    seed_value: u64,
) -> Result<DomainPayload> {
    let pack: Domain = load_domain_pack(domain_root).with_context(|| {
        format!(
            "loading domain pack '{domain_id}' from {}",
            domain_root.display()
        )
    })?;

    let opts = GenerationOptions {
        seed: seed_value,
        max_attempts: GENERATOR_MAX_ATTEMPTS,
    };
    let grid =
        generate(&pack, opts).with_context(|| format!("generating grid for '{domain_id}'"))?;

    let locale = pack.metadata.default_locale.as_str();
    let snapshot = grid.snapshot_with_entities(locale, seed_value, &pack.entities);

    Ok(DomainPayload {
        payload: serde_json::to_value(snapshot).context("serialising grid snapshot")?,
        version: pack.metadata.version.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn dry_run_without_conn_succeeds_for_real_pack() {
        // Use the in-repo paris-metro pack as a fixture. With `conn = None` we
        // exercise the load + generate path without touching the DB.
        let workspace_root = workspace_root();
        let pack_root = workspace_root.join("domains/paris-metro");
        if !pack_root.exists() {
            eprintln!("skipping: paris-metro pack not present");
            return;
        }
        let domain = DiscoveredDomain {
            id: "paris-metro".to_owned(),
            root: pack_root,
        };
        let date = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let outcome = generate_for_domain(&domain, date, None).await.unwrap();
        assert_eq!(outcome, GenerationOutcome::DryRun);
    }

    #[tokio::test]
    async fn missing_pack_surfaces_error_without_panic() {
        let domain = DiscoveredDomain {
            id: "mock-domain".to_owned(),
            root: PathBuf::from("/nonexistent"),
        };
        let date = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let res = generate_for_domain(&domain, date, None).await;
        assert!(res.is_err(), "missing pack must propagate an error");
    }

    fn workspace_root() -> PathBuf {
        // CARGO_MANIFEST_DIR points at `worker/`, so the workspace root is its parent.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("worker/ must have a parent (workspace root)")
            .to_path_buf()
    }
}
