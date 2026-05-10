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

use axum::{
    http::{header::HeaderName, HeaderValue},
    routing::{get, post},
    Router,
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;

const CSP: &str = "default-src 'self'; \
img-src 'self' data:; \
style-src 'self' 'unsafe-inline'; \
script-src 'self'; \
connect-src 'self'; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'self'";

/// Build the full Axum router. Tests instantiate this directly so they exercise the
/// exact same chain as production (minus the actual TCP listener).
#[must_use]
pub fn build_router(state: AppState) -> Router {
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

    Router::new()
        .nest("/api", api_with_session)
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(CSP),
        ))
        .with_state(state)
}
