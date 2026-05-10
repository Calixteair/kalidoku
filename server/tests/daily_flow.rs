//! Integration test for issue #26 — happy-path of the daily mode:
//!
//! 1. spin up a throwaway Postgres via `testcontainers`,
//! 2. apply the SeaORM migrations through `kalidoku_server::migrations::Migrator`,
//! 3. seed `domains` + a `daily` grid with a known payload,
//! 4. drive `POST /api/games`, `POST /api/games/{id}/play`, `GET /api/games/{id}/result`
//!    against the in-process router via `tower::ServiceExt::oneshot`.
//!
//! Note on cookies: the session middleware mints a `kd_device` cookie on the very
//! first request and persists the row in DB. We replay that cookie on the next
//! requests so they share the same `device_id` (and the play_token issued by
//! `start_game` matches).

#![cfg(not(target_os = "windows"))]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use kalidoku_server::{
    build_router_with_options,
    config::AppConfig,
    db,
    entities::{domains as domain_entity, grids},
    migrations::Migrator,
    state::{AppState, DomainSummary},
    RouterOptions,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};
use sea_orm_migration::MigratorTrait;
use serde_json::{json, Value};
use std::collections::HashMap;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

/// Build the full router + a real Postgres-backed state. Returned tuple keeps the
/// container alive for the lifetime of the test (drop closes it cleanly).
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

    let mut domains_map = HashMap::new();
    domains_map.insert(
        "paris-metro".to_string(),
        DomainSummary {
            id: "paris-metro".into(),
            name_fr: "Métro de Paris".into(),
            name_en: "Paris Metro".into(),
            version: "0.1.0".into(),
            description: None,
            available_modes: vec!["daily".into()],
        },
    );
    let cfg = AppConfig::test_fixture();
    let state = AppState::new(cfg)
        .with_db(conn.clone())
        .with_domains(domains_map);
    let app = build_router_with_options(state, RouterOptions { rate_limit: false });
    (app, conn, pg)
}

/// Insert one `domains` row + one `daily` grid with a tiny entity index and a
/// known cell-(0,0) candidate. Returns the entity name we'll use as the answer.
async fn seed_domain_and_grid(conn: &DatabaseConnection) -> (Uuid, String) {
    let now = chrono::Utc::now();

    let dom = domain_entity::ActiveModel {
        id: Set("paris-metro".to_string()),
        version: Set("0.1.0".into()),
        active: Set(true),
        metadata: Set(json!({})),
        created_at: Set(now.into()),
    };
    domain_entity::Entity::insert(dom)
        .exec(conn)
        .await
        .expect("insert domain");

    // The smallest payload `routes::games` will accept:
    //   - `rows` and `cols` arrays of predicate descriptors,
    //   - `entities` index with at least one entry (id, name, aliases),
    //   - `candidates` 3x3 of arrays of entity ids.
    let payload = json!({
        "rows": [
            { "id": "r0", "family": "demo", "label": "row 0" },
            { "id": "r1", "family": "demo", "label": "row 1" },
            { "id": "r2", "family": "demo", "label": "row 2" }
        ],
        "cols": [
            { "id": "c0", "family": "demo", "label": "col 0" },
            { "id": "c1", "family": "demo", "label": "col 1" },
            { "id": "c2", "family": "demo", "label": "col 2" }
        ],
        "entities": [
            { "id": "ent_a", "name": "Châtelet", "aliases": ["chatelet"] }
        ],
        "candidates": [
            [["ent_a"], [], []],
            [[], [], []],
            [[], [], []]
        ]
    });

    let grid_id = Uuid::now_v7();
    let grid = grids::ActiveModel {
        id: Set(grid_id),
        domain: Set("paris-metro".into()),
        mode: Set("daily".into()),
        publish_at: Set(now.into()),
        payload: Set(payload),
        seed: Set(42),
        created_at: Set(now.into()),
    };
    grid.insert(conn).await.expect("insert grid");

    (grid_id, "Châtelet".to_string())
}

/// Pull every `set-cookie` line out of a response, keep only the `kd_device=...`
/// piece, return it ready to splice into the next request's `cookie` header.
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

#[tokio::test]
#[ignore = "spawns a Postgres container; opt in with `cargo test -- --ignored`"]
async fn daily_start_play_result_returns_201_200_200() {
    let (app, conn, _pg) = boot_app().await;
    let (_grid_id, answer) = seed_domain_and_grid(&conn).await;

    // 1. POST /api/games — anonymous, no cookie. The middleware will mint and
    //    return a `kd_device` cookie which we'll reuse on subsequent requests.
    let start_body = json!({ "domain": "paris-metro", "mode": "daily" }).to_string();
    let start_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/games")
                .header("content-type", "application/json")
                .body(Body::from(start_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        start_resp.status(),
        StatusCode::CREATED,
        "expected 201 Created on POST /api/games"
    );
    let device_cookie =
        extract_device_cookie(&start_resp).expect("session middleware must mint kd_device");

    let start_json = body_to_json(start_resp).await;
    let game_id = start_json
        .get("game")
        .and_then(|g| g.get("id"))
        .and_then(|v| v.as_str())
        .expect("game.id")
        .to_string();
    let play_token = start_json
        .get("playToken")
        .and_then(|t| t.get("token"))
        .and_then(|v| v.as_str())
        .expect("playToken.token")
        .to_string();

    // 2. POST /api/games/{id}/play with the cookie + token + a known answer.
    let play_body = json!({
        "cell": { "row": 0, "col": 0 },
        "answer": answer,
        "playToken": play_token,
    })
    .to_string();
    let play_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/games/{game_id}/play"))
                .header("content-type", "application/json")
                .header("cookie", &device_cookie)
                .body(Body::from(play_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        play_resp.status(),
        StatusCode::OK,
        "expected 200 OK on POST /api/games/{{id}}/play"
    );
    let play_json = body_to_json(play_resp).await;
    assert_eq!(
        play_json.get("ok"),
        Some(&Value::Bool(true)),
        "play response should mark the answer as correct: {play_json}"
    );

    // 3. GET /api/games/{id}/result. The handler refuses while the game is still
    //    active, so we abandon the game first to flip the status — this exercises
    //    the same code path as a fully completed run, just without the 8 extra
    //    plays the dataset would need to seed.
    let abandon_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/games/{game_id}/abandon"))
                .header("cookie", &device_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        abandon_resp.status(),
        StatusCode::OK,
        "expected 200 OK on /abandon"
    );

    let result_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/games/{game_id}/result"))
                .header("cookie", &device_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        result_resp.status(),
        StatusCode::OK,
        "expected 200 OK on GET /api/games/{{id}}/result"
    );

    let result_json = body_to_json(result_resp).await;
    assert!(
        result_json.get("solutionsByCell").is_some(),
        "result view must expose solutionsByCell once the game is finished"
    );
    let summary = result_json
        .get("summary")
        .expect("result.summary block must exist");
    assert_eq!(summary.get("solved"), Some(&json!(1)));
}
