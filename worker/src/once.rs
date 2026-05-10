//! `--once` mode : iterate over discovered domains, generate today's daily grid
//! when missing, persist it.
//!
//! Designed to remain useful even when:
//! - `DATABASE_URL` is unset (local dev) → log warn + exit 0.
//! - The `grids` table is missing (agent C migrations pending) → log warn + exit 0.
//! - `core::generator::generate` is still unimplemented → log error + skip the
//!   domain (we use `catch_unwind` to survive the `unimplemented!()` panic).

use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};

use crate::{
    db::{self, DbState},
    domain_pack::{self, DiscoveredDomain},
    persist::{self, GridRecord},
    seed,
};

const DAILY_PUBLISH_HOUR_UTC: u32 = 0;
const DAILY_PUBLISH_MINUTE_UTC: u32 = 1;

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

#[derive(Debug)]
enum GenerationOutcome {
    Inserted,
    AlreadyPresent,
    DryRun,
}

async fn list_active_domains() -> Result<Vec<DiscoveredDomain>> {
    // For now we only support the filesystem fallback. When agent C ships the
    // `domains` table + an `active=true` flag we'll wire it in here.
    domain_pack::discover(&domain_pack::default_root())
}

async fn generate_for_domain(
    domain: &DiscoveredDomain,
    date: NaiveDate,
    conn: Option<&sea_orm::DatabaseConnection>,
) -> Result<GenerationOutcome> {
    let iso_date = date.format("%Y-%m-%d").to_string();
    let publish_at = Utc.from_utc_datetime(
        &date
            .and_hms_opt(DAILY_PUBLISH_HOUR_UTC, DAILY_PUBLISH_MINUTE_UTC, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid publish_at for {iso_date}"))?,
    );

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

    let payload = match generate_payload(&domain.id, &domain.root, derived_seed).await {
        Ok(p) => p,
        Err(GenerationError::Unimplemented) => {
            // TODO: enable once core::generator::generate is implemented (agent A).
            tracing::error!(
                domain = %domain.id,
                "core::generator::generate is not implemented yet — skipping this domain"
            );
            return Err(anyhow::anyhow!(
                "core::generator::generate unimplemented (agent A)"
            ));
        }
        Err(GenerationError::Other(e)) => return Err(e),
    };

    let Some(c) = conn else {
        tracing::warn!(
            domain = %domain.id,
            "DATABASE_URL not set, skipping INSERT (dry-run)"
        );
        return Ok(GenerationOutcome::DryRun);
    };

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

#[derive(Debug)]
enum GenerationError {
    Unimplemented,
    Other(anyhow::Error),
}

/// Bridge to `core::*`. Currently the loader and the generator are stubs that
/// `unimplemented!()`, so we run them inside `catch_unwind` and downgrade the
/// panic into `GenerationError::Unimplemented`.
async fn generate_payload(
    domain_id: &str,
    domain_root: &std::path::Path,
    seed_value: u64,
) -> Result<serde_json::Value, GenerationError> {
    let domain_id = domain_id.to_owned();
    let domain_root = domain_root.to_path_buf();

    let join = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            generate_via_core(&domain_id, &domain_root, seed_value)
        }))
    })
    .await
    .map_err(|e| GenerationError::Other(anyhow::anyhow!("blocking task crashed: {e}")))?;

    match join {
        Ok(Ok(payload)) => Ok(payload),
        Ok(Err(e)) => Err(GenerationError::Other(e)),
        Err(_) => Err(GenerationError::Unimplemented),
    }
}

fn generate_via_core(
    _domain_id: &str,
    _domain_root: &std::path::Path,
    _seed_value: u64,
) -> Result<serde_json::Value> {
    // TODO: enable once core::domain::loader::load_domain_pack and
    // core::generator::generate are implemented by agent A.
    //
    // Expected wiring:
    //   let pack = kalidoku_core::domain::loader::load_domain_pack(domain_root)?;
    //   let opts = kalidoku_core::generator::GenerationOptions {
    //       seed: seed_value,
    //       ..Default::default()
    //   };
    //   let grid = kalidoku_core::generator::generate(&pack, opts)?;
    //   Ok(serialize_grid(&grid))
    //
    // For now we deliberately call the stub so that `catch_unwind` reports
    // `Unimplemented` to the caller, which logs cleanly and exits with a TODO.
    Err(anyhow::anyhow!(
        "core::generator::generate not wired yet — see TODO in worker/src/once.rs"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    #[ignore] // TODO: enable once core::generator::generate is implemented
    async fn once_generates_when_db_ready() {
        // Requires a real DB + agent A's generator. Wired as soon as both
        // dependencies are in.
    }

    #[tokio::test]
    async fn dry_run_when_no_conn_reports_failure_without_panic() {
        // Without a DB and with the generator stub, `generate_for_domain`
        // returns Err(unimplemented). We assert it doesn't panic and doesn't
        // try to write anywhere.
        let domain = DiscoveredDomain {
            id: "mock-domain".to_owned(),
            root: PathBuf::from("/nonexistent"),
        };
        let date = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let res = generate_for_domain(&domain, date, None).await;
        assert!(res.is_err(), "stub generator must propagate an error");
    }

    #[tokio::test]
    async fn payload_generation_surfaces_unimplemented_cleanly() {
        // The current core stub triggers `Err(...)` (not a panic). Either way,
        // we must observe a `GenerationError::Other` and not crash the runtime.
        let res = generate_payload("paris-metro", std::path::Path::new("."), 42).await;
        assert!(res.is_err());
    }
}
