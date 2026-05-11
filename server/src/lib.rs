//! kalidoku-server library entry point. Exposing modules from a `lib.rs` lets the
//! integration tests in `tests/` exercise the same router builder used by `main`.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]
#![allow(
    elided_lifetimes_in_paths,
    clippy::double_must_use,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions
)]

pub mod altcha;
pub mod auth;
pub mod cache;
pub mod config;
pub mod db;
pub mod duel_sig;
pub mod entities;
pub mod error;
pub mod middleware;
pub mod migrations;
pub mod play_token;
pub mod quota;
pub mod repos;
pub mod routes;
pub mod services;
pub mod state;
pub mod telemetry;

use std::sync::Arc;

use axum::{
    http::{header::HeaderName, HeaderValue},
    routing::{get, post},
    Router,
};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::middleware::headers::{
    CSP, HSTS, PERMISSIONS_POLICY, REFERRER, X_CONTENT_TYPE, X_FRAME,
};
use crate::state::AppState;

/// Build the full Axum router with rate-limit enabled. Used by `main.rs` in production.
#[must_use]
pub fn build_router(state: AppState) -> Router {
    build_router_with_options(state, RouterOptions::default())
}

/// Knobs for `build_router_with_options`. Tests skip the per-IP rate-limit because
/// `tower::ServiceExt::oneshot` carries no real peer address and the smart IP extractor
/// would 500 on a missing key.
#[derive(Debug, Clone, Copy)]
pub struct RouterOptions {
    pub rate_limit: bool,
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self { rate_limit: true }
    }
}

#[must_use]
pub fn build_router_with_options(state: AppState, opts: RouterOptions) -> Router {
    let api = Router::new()
        // health + altcha
        .route("/health", get(routes::health::get))
        .route("/altcha/challenge", get(routes::altcha::get))
        // auth
        .route("/auth/login", get(routes::auth::login))
        .route("/auth/callback", get(routes::auth::callback))
        .route("/auth/logout", post(routes::auth::logout))
        .route("/me", get(routes::auth::me))
        // domains
        .route("/domains", get(routes::domains::list))
        .route(
            "/domains/:domain/autocomplete",
            get(routes::domains::autocomplete),
        )
        // grids
        .route("/grids/:domain/today", get(routes::games::today))
        // games
        .route("/games", post(routes::games::start_game))
        .route("/games/:game_id/play", post(routes::games::play))
        .route("/games/:game_id/abandon", post(routes::games::abandon))
        .route("/games/:game_id/result", get(routes::games::result))
        // leaderboard
        .route(
            "/leaderboard/:domain/today",
            get(routes::leaderboard::today),
        )
        // duels
        .route("/duels", post(routes::duels::create))
        .route("/duels/:duel_id", get(routes::duels::view));

    let api_with_session = api.layer(axum::middleware::from_fn_with_state(
        state.clone(),
        middleware::session::middleware,
    ));

    // Per-IP rate-limit. SmartIpKeyExtractor reads X-Forwarded-For when behind a
    // trusted proxy (NPM in our case), falls back to the direct peer address otherwise.
    // Per-device tightening (cookie-keyed) is tracked in issue #21 follow-up.
    let rate_limit_layer = opts.rate_limit.then(|| {
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(middleware::governor::IP_PERIOD.as_secs().max(1))
                .burst_size(middleware::governor::IP_BURST)
                .finish()
                .expect("static governor config"),
        );
        GovernorLayer {
            config: governor_conf,
        }
    });

    let router = Router::new()
        .nest("/api", api_with_session)
        // Security headers — applied outermost so even error responses get them.
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static(HSTS),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static(X_CONTENT_TYPE),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static(X_FRAME),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static(REFERRER),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(PERMISSIONS_POLICY),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(CSP),
        ))
        // Request tracing — JSON spans, captured by tracing-subscriber in telemetry.rs.
        .layer(middleware::trace::layer())
        .with_state(state);

    // Apply rate-limit last (innermost layer-wise = topmost in Tower's stack semantics).
    if let Some(layer) = rate_limit_layer {
        router.layer(layer)
    } else {
        router
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;

    fn test_router() -> Router {
        // Test-mode router: rate-limit disabled because oneshot has no peer IP.
        // Rate-limit is exercised by an integration test that uses an actual TCP listener.
        build_router_with_options(AppState::for_tests(), RouterOptions { rate_limit: false })
    }

    #[tokio::test]
    async fn health_endpoint_returns_200() {
        let app = test_router();
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
    }

    #[tokio::test]
    async fn health_endpoint_carries_security_headers() {
        let app = test_router();
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
        for h in [
            "permissions-policy",
            "content-security-policy",
            "strict-transport-security",
            "x-frame-options",
            "x-content-type-options",
            "referrer-policy",
        ] {
            assert!(headers.contains_key(h), "{h} header missing");
        }
    }

    #[tokio::test]
    async fn rate_limit_layer_can_be_built() {
        // Sanity check that the production options actually wires the rate-limit layer.
        // We don't exercise it under burst here (oneshot has no peer IP); a follow-up
        // integration test in tests/ should hit it via an actual TCP listener.
        let _ =
            build_router_with_options(AppState::for_tests(), RouterOptions { rate_limit: true });
    }
}
