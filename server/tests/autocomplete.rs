//! Integration tests for `GET /api/domains/{id}/autocomplete`.
//!
//! The autocomplete endpoint is a thin Meilisearch proxy. We boot the router
//! with a [`MeiliClient`] pointed at a `wiremock` mock server (no real Meili
//! container needed — the contract we care about is purely HTTP-level).

#![cfg(not(target_os = "windows"))]

use std::collections::HashMap;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kalidoku_server::{
    build_router_with_options,
    config::AppConfig,
    services::meili::{MeiliClient, SearchHit},
    state::{AppState, DomainSummary},
    RouterOptions,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_KEY: &str = "test-meili-master-key";

/// Stand up the router + a wiremock-backed Meili client. The mock URL is what
/// the server will POST to, so the path prefix in the mocks must match.
async fn boot_app_with_meili(mock: &MockServer) -> axum::Router {
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
    let meili = MeiliClient::new(mock.uri(), TEST_KEY.into());
    let state = AppState::new(cfg)
        .with_domains(domains_map)
        .with_meili(meili);
    build_router_with_options(state, RouterOptions { rate_limit: false })
}

/// Convenience: extract the body as a JSON Value. Panics on failure — fine for
/// tests where any malformed body is a regression.
async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn autocomplete_returns_hits_from_meili() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/indexes/paris-metro/search"))
        .and(header(
            "authorization",
            format!("Bearer {TEST_KEY}").as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "hits": [
                { "id": "metro:1", "name": "Châtelet" },
                { "id": "metro:2", "name": "Châtelet - Les Halles" },
                { "id": "metro:3", "name": "Châteaurouge" },
            ],
            "query": "Châtelet",
            "processingTimeMs": 1,
            "limit": 5,
            "offset": 0,
            "estimatedTotalHits": 3,
        })))
        .mount(&mock)
        .await;

    let app = boot_app_with_meili(&mock).await;

    // Note: axum/tower percent-decode the path; the raw URL is enough for the
    // query parser to populate `q` & `limit`.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/domains/paris-metro/autocomplete?q=Ch%C3%A2telet&limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let hits: Vec<SearchHit> = serde_json::from_value(body).unwrap();
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].id, "metro:1");
    assert_eq!(hits[0].name, "Châtelet");
}

#[tokio::test]
async fn autocomplete_rejects_empty_q_with_400() {
    let mock = MockServer::start().await;
    let app = boot_app_with_meili(&mock).await;

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/domains/paris-metro/autocomplete?q=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn autocomplete_rejects_overlong_q_with_400() {
    let mock = MockServer::start().await;
    let app = boot_app_with_meili(&mock).await;

    let too_long: String = "a".repeat(101);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/domains/paris-metro/autocomplete?q={too_long}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn autocomplete_clamps_huge_limit_to_max() {
    // The mock asserts the body Meili receives has `limit == 20`, which is
    // proof the handler clamped the user-supplied 999.
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/indexes/paris-metro/search"))
        .and(wiremock::matchers::body_partial_json(
            json!({ "limit": 20 }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "hits": [],
            "query": "x",
            "processingTimeMs": 1,
            "limit": 20,
            "offset": 0,
            "estimatedTotalHits": 0,
        })))
        .mount(&mock)
        .await;

    let app = boot_app_with_meili(&mock).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/domains/paris-metro/autocomplete?q=x&limit=999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn autocomplete_returns_502_when_meili_500() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/indexes/paris-metro/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal boom"))
        .mount(&mock)
        .await;

    let app = boot_app_with_meili(&mock).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/domains/paris-metro/autocomplete?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn autocomplete_returns_503_when_meili_not_configured() {
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
    let state = AppState::new(AppConfig::test_fixture()).with_domains(domains_map);
    let app = build_router_with_options(state, RouterOptions { rate_limit: false });

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/domains/paris-metro/autocomplete?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn autocomplete_returns_404_for_unknown_domain() {
    let mock = MockServer::start().await;
    let app = boot_app_with_meili(&mock).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/domains/no-such-domain/autocomplete?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
