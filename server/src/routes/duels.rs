//! Duel creation + viewing. Duel grid generation is owned by the worker queue
//! (agent B), so the create handler currently returns 501 — the route exists so the
//! front can flip it on once the worker publishes a `solo`/`duel` grid.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateDuelRequest {
    pub domain: String,
    #[serde(default)]
    pub altcha_solution: Option<String>,
}

#[derive(Serialize)]
pub struct CreateDuelResponse {
    pub duel_id: Uuid,
    pub share_url: String,
    pub grid_id: Uuid,
}

pub async fn create(
    State(_state): State<AppState>,
    Json(_body): Json<CreateDuelRequest>,
) -> ApiResult<Json<CreateDuelResponse>> {
    Err(ApiError::NotImplemented(
        "duel grid generation waiting for worker queue (agent-b)",
    ))
}

#[derive(Deserialize)]
pub struct ViewQuery {
    pub sig: String,
}

#[derive(Serialize)]
pub struct DuelView {
    pub duel_id: Uuid,
    pub grid_id: Uuid,
    pub players: Vec<serde_json::Value>,
}

pub async fn view(
    State(_state): State<AppState>,
    Path(_duel_id): Path<Uuid>,
    Query(_q): Query<ViewQuery>,
) -> ApiResult<Json<DuelView>> {
    Err(ApiError::NotImplemented(
        "duel view waits for create flow + summary aggregation",
    ))
}
