//! Integration test for issue #24 — confirms the response chain attaches the
//! `Permissions-Policy` header (and friends) to every response, including the
//! cheapest path: `GET /api/health`.
//!
//! `Permissions-Policy` and the `tower_http::trace` layer were already wired in
//! `server::build_router_with_options`, so this test guards against a regression
//! where someone accidentally drops a layer from the stack.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use kalidoku_server::{
    build_router_with_options, config::AppConfig, state::AppState, RouterOptions,
};
use tower::ServiceExt;

fn router_without_rate_limit() -> axum::Router {
    // `oneshot` carries no peer IP; the real server uses connect-info to expose one.
    // We use the public `AppConfig::test_fixture()` rather than the in-crate
    // `AppState::for_tests()` (which is `#[cfg(test)]`-gated and therefore invisible
    // to integration tests living under `tests/`).
    let state = AppState::new(AppConfig::test_fixture());
    build_router_with_options(state, RouterOptions { rate_limit: false })
}

#[tokio::test]
async fn health_response_carries_permissions_policy_header() {
    let app = router_without_rate_limit();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let value = resp
        .headers()
        .get("permissions-policy")
        .expect("permissions-policy header must be present");
    let s = value.to_str().expect("header is ASCII");
    // We don't pin the exact policy string here — it lives in middleware/headers.rs
    // and may grow as we lock down more APIs. We only assert that at least one
    // sensitive feature is denied with the empty allowlist syntax.
    assert!(
        s.contains("camera=()") && s.contains("microphone=()"),
        "permissions-policy missing core denials: {s}"
    );
}

#[tokio::test]
async fn health_response_carries_full_security_header_set() {
    // Belt-and-braces: confirm the rest of the security headers documented in
    // CLAUDE.md §4 are still emitted alongside Permissions-Policy.
    let app = router_without_rate_limit();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let headers = resp.headers();
    for name in [
        "permissions-policy",
        "content-security-policy",
        "strict-transport-security",
        "x-frame-options",
        "x-content-type-options",
        "referrer-policy",
    ] {
        assert!(
            headers.contains_key(name),
            "expected response header `{name}` to be set by the security layer stack"
        );
    }
}
