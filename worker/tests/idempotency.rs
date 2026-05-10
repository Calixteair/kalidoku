//! Integration test : the `--once` pipeline is idempotent on `(domain, date)`.
//!
//! Spins up a temporary Postgres via `testcontainers`, runs the migrations
//! exposed by `kalidoku-server`, then calls `once::generate_for_domain` twice
//! for the same `(domain, date)` and asserts that exactly one row lands in
//! the `grids` table.
//!
//! Skipped at runtime when no Docker socket is reachable (typical CI without
//! a docker-in-docker rig); the test still type-checks so the wiring stays
//! honest.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
use sea_orm_migration::MigratorTrait;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

use kalidoku_server::migrations::Migrator;
use kalidoku_worker::{domain_pack::DiscoveredDomain, once};

#[tokio::test]
async fn once_pipeline_is_idempotent_per_domain_and_date() {
    if !docker_available().await {
        eprintln!("skipping: no reachable Docker socket");
        return;
    }

    let pack_root = paris_metro_root();
    if !pack_root.exists() {
        eprintln!(
            "skipping: paris-metro pack not present at {}",
            pack_root.display()
        );
        return;
    }

    let container = match Postgres::default()
        .with_db_name("kalidoku_test")
        .with_user("kalidoku")
        .with_password("kalidoku")
        .start()
        .await
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skipping: failed to start Postgres container: {e}");
            return;
        }
    };

    let host = container
        .get_host()
        .await
        .expect("container host")
        .to_string();
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("container port");
    let url = format!("postgres://kalidoku:kalidoku@{host}:{port}/kalidoku_test");

    let conn = connect(&url).await;
    Migrator::up(&conn, None).await.expect("running migrations");

    let domain = DiscoveredDomain {
        id: "paris-metro".to_owned(),
        root: pack_root,
    };
    let date = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();

    // First pass: must INSERT exactly one row.
    let first = once::generate_for_domain(&domain, date, Some(&conn))
        .await
        .expect("first generation");
    assert_eq!(
        first,
        once::GenerationOutcome::Inserted,
        "first call should insert"
    );

    // Second pass with the same (domain, date): must short-circuit on the
    // `has_daily_grid` probe and report `AlreadyPresent` without inserting.
    let second = once::generate_for_domain(&domain, date, Some(&conn))
        .await
        .expect("second generation");
    assert_eq!(
        second,
        once::GenerationOutcome::AlreadyPresent,
        "second call should be a noop"
    );

    let count = count_grids(&conn, &domain.id, "daily").await;
    assert_eq!(count, 1, "exactly one grid row must exist after two passes");
}

async fn docker_available() -> bool {
    // Cheapest probe: try to start the smallest possible container and bail
    // out on any error. We deliberately swallow the error — most CI runners
    // without docker-in-docker land here and the test should skip gracefully.
    Postgres::default()
        .with_db_name("kalidoku_probe")
        .with_user("u")
        .with_password("p")
        .start()
        .await
        .is_ok()
}

async fn connect(url: &str) -> DatabaseConnection {
    let mut opts = ConnectOptions::new(url.to_owned());
    opts.sqlx_logging(false);
    Database::connect(opts).await.expect("connect to test DB")
}

async fn count_grids(conn: &DatabaseConnection, domain: &str, mode: &str) -> i64 {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT COUNT(*)::bigint AS n FROM grids WHERE domain = $1 AND mode = $2",
        [domain.into(), mode.into()],
    );
    let row = conn
        .query_one(stmt)
        .await
        .expect("count query")
        .expect("count row");
    row.try_get::<i64>("", "n").expect("n column")
}

fn paris_metro_root() -> PathBuf {
    workspace_root().join("domains/paris-metro")
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at `worker/`, the workspace root is its parent.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("worker/ must have a parent (workspace root)")
        .to_path_buf()
}
