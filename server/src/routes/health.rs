//! `/api/health` — liveness/readiness probe.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
    time: String,
}

pub async fn get() -> impl IntoResponse {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        time: chrono::Utc::now().to_rfc3339(),
    })
}
