//! Integration test for solo mode.
//!
//! Boots a Postgres container, points `AppConfig.domains_root` at the in-repo
//! paris-metro pack (via `CARGO_MANIFEST_DIR/../domains`), and asserts that
//! `POST /api/games {mode:"solo"}` generates a fresh grid on the fly, returns
//! a seed, and accepts a follow-up request with the same seed to regenerate
//! the same grid.

#![cfg(not(target_os = "windows"))]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use kalidoku_server::{
    build_router_with_options,
    config::AppConfig,
    db,
    entities::domains as domain_entity,
    migrations::Migrator,
    state::{AppState, DomainSummary},
    RouterOptions,
};
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait};
use sea_orm_migration::MigratorTrait;
use serde_json::{json, Value};
use std::collections::HashMap;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;

async fn boot_app() -> (
    axum::Router,
    DatabaseConnection,
    testcontainers::ContainerAsync<PgImage>,
) {
    let pg = PgImage::default()
        .start()
        .await
        .expect("postgres container starts");
    let host = pg.get_host().await.expect("pg host");
    let port = pg.get_host_port_ipv4(5432).await.expect("pg port mapping");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let conn = db::connect(&url).await.expect("db pool");
    Migrator::up(&conn, None).await.expect("migrations apply");

    // Seed the domain row so the FK on grids.domain is satisfied.
    let dom = domain_entity::ActiveModel {
        id: Set("paris-metro".to_string()),
        version: Set("0.3.0".into()),
        active: Set(true),
        metadata: Set(json!({})),
        created_at: Set(chrono::Utc::now().into()),
    };
    domain_entity::Entity::insert(dom)
        .exec(&conn)
        .await
        .expect("insert domain");

    let mut domains_map = HashMap::new();
    domains_map.insert(
        "paris-metro".to_string(),
        DomainSummary {
            id: "paris-metro".into(),
            name_fr: "Métro de Paris".into(),
            name_en: "Paris Metro".into(),
            version: "0.3.0".into(),
            description: None,
            available_modes: vec!["daily".into(), "solo".into()],
        },
    );
    let mut cfg = AppConfig::test_fixture();
    cfg.domains_root = pack_root_for_tests().to_string_lossy().into_owned();
    let state = AppState::new(cfg)
        .with_db(conn.clone())
        .with_domains(domains_map);
    let app = build_router_with_options(state, RouterOptions { rate_limit: false });
    (app, conn, pg)
}

fn extract_device_cookie(resp: &axum::response::Response) -> Option<String> {
    for v in resp.headers().get_all(header::SET_COOKIE).iter() {
        let s = v.to_str().ok()?;
        for piece in s.split(';') {
            let p = piece.trim();
            if p.starts_with("kd_device=") {
                return Some(p.to_string());
            }
        }
    }
    None
}

async fn body_to_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    serde_json::from_slice(&bytes).expect("json body")
}

fn pack_root_for_tests() -> std::path::PathBuf {
    // `env!` resolves at compile-time so we don't need std::env::var (which is
    // banned by clippy.toml outside the config service).
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("domains")
}

#[tokio::test]
#[ignore = "spawns a Postgres container; opt in with `cargo test -- --ignored`"]
async fn solo_returns_201_with_fresh_seed_and_grid_payload() {
    let (app, _conn, _pg) = boot_app().await;

    let body = json!({ "domain": "paris-metro", "mode": "solo" }).to_string();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/games")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    extract_device_cookie(&resp).expect("middleware mints kd_device");

    let j = body_to_json(resp).await;
    let grid = j.get("grid").expect("grid in response");
    assert_eq!(grid.get("mode").and_then(Value::as_str), Some("solo"));
    let seed = grid
        .get("seed")
        .and_then(Value::as_u64)
        .expect("seed exposed on solo");
    assert!(
        seed < (1u64 << 32),
        "expected a short 32-bit seed, got {seed}"
    );

    assert!(grid.get("rows").and_then(Value::as_array).unwrap().len() == 3);
    assert!(grid.get("cols").and_then(Value::as_array).unwrap().len() == 3);
    assert!(
        grid.get("candidatesCount")
            .and_then(Value::as_array)
            .unwrap()
            .len()
            == 9
    );
}

#[tokio::test]
#[ignore = "spawns a Postgres container; opt in with `cargo test -- --ignored`"]
async fn solo_with_explicit_seed_is_deterministic() {
    let (app, _conn, _pg) = boot_app().await;

    let send = |seed: u64| {
        let app = app.clone();
        async move {
            let body = json!({ "domain": "paris-metro", "mode": "solo", "seed": seed }).to_string();
            let resp = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/games")
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::CREATED);
            body_to_json(resp).await
        }
    };

    let a = send(31_337).await;
    let b = send(31_337).await;

    let row_labels = |j: &Value| -> Vec<String> {
        j["grid"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["label"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        row_labels(&a),
        row_labels(&b),
        "same seed → same predicates"
    );
    assert_eq!(a["grid"]["seed"], json!(31_337_u64));
}
