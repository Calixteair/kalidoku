//! `/api/games` family — start, play, abandon, result.
//!
//! The core validator (`core::validator::validate_answer`) is the one true source of
//! truth for what counts as a Match/Wrong. Until agent A publishes a stable signature
//! to load Domain + Grid from DB-stored payloads, the play handler returns 501.

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::altcha;
use crate::auth::AuthContext;
use crate::entities::{games, grids};
use crate::error::{ApiError, ApiResult};
use crate::play_token::{self, PlayTokenPayload};
use crate::quota::{self, GameMode};
use crate::services::altcha as altcha_service;
use crate::state::AppState;

const MAX_MISTAKES: i32 = 3;
const MAX_SCORE: i32 = 9 * 100;

// ----- start_game -----

#[derive(Deserialize)]
pub struct StartGameRequest {
    pub domain: String,
    pub mode: String,
    #[serde(default)]
    pub duel_grid_id: Option<Uuid>,
    #[serde(default)]
    pub altcha_solution: Option<String>,
}

#[derive(Serialize)]
pub struct StartGameGame {
    pub id: Uuid,
    pub started_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct StartGamePlayToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct StartGameResponse {
    pub game: StartGameGame,
    pub grid: PublicGrid,
    pub play_token: StartGamePlayToken,
}

#[derive(Serialize)]
pub struct PublicGrid {
    pub id: Uuid,
    pub domain: String,
    pub mode: String,
    pub publish_at: chrono::DateTime<Utc>,
    pub rows: Vec<PredicateLabel>,
    pub cols: Vec<PredicateLabel>,
    pub mistakes_allowed: i32,
}

#[derive(Serialize)]
pub struct PredicateLabel {
    pub id: String,
    pub family: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

pub async fn start_game(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<StartGameRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<StartGameResponse>)> {
    let mode = GameMode::parse(&body.mode)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown mode {}", body.mode)))?;
    if !state.domains.contains_key(&body.domain) {
        return Err(ApiError::NotFound("domain"));
    }
    let device_id = ctx.device_id.ok_or(ApiError::Unauthorised)?;

    // Anti-bot: when explicitly demanded by the front (anonymous user) or always for duels.
    let altcha_required = matches!(mode, GameMode::Duel) || !ctx.is_authenticated;
    if altcha_required {
        let solution = body
            .altcha_solution
            .as_deref()
            .ok_or(ApiError::AltchaRequired)?;
        let key = state.config.altcha_hmac_key.as_bytes();
        let parsed = altcha::decode_solution(solution).map_err(|_| ApiError::AltchaRequired)?;
        if !altcha::verify_solution(&parsed, key) {
            return Err(ApiError::AltchaRequired);
        }
        // replay-protection (best-effort; failure to talk to Redis falls open with a warn).
        if let Some(redis) = state.redis.as_ref() {
            altcha_service::guard_replay(redis, &parsed.challenge).await?;
        }
    }

    // Quota check (premium short-circuits via the future users.premium_active flag).
    quota::check(false, mode, 0).map_err(|e| match e {
        quota::QuotaError::PremiumRequired => ApiError::PaymentRequired,
        _ => ApiError::PaymentRequired,
    })?;

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;

    // Find a grid for the requested mode + domain. For daily, the worker must have
    // published one for today; for solo/duel agent C will trigger the worker queue.
    let grid_row = match mode {
        GameMode::Daily => {
            grids::Entity::find()
                .filter(grids::Column::Domain.eq(body.domain.clone()))
                .filter(grids::Column::Mode.eq("daily"))
                .order_by_desc(grids::Column::PublishAt)
                .one(db.as_ref())
                .await?
        }
        GameMode::Solo | GameMode::Duel => {
            return Err(ApiError::NotImplemented(
                "solo/duel grid generation waiting for worker queue (agent-b)",
            ));
        }
    };
    let grid = grid_row.ok_or(ApiError::NotFound("no grid published yet"))?;

    // Idempotency: if a row already exists for this (grid, device) we 409.
    let existing = games::Entity::find()
        .filter(games::Column::GridId.eq(grid.id))
        .filter(games::Column::DeviceId.eq(device_id))
        .one(db.as_ref())
        .await?;
    if let Some(g) = existing {
        if g.status == "active" {
            // Re-emit a play token so the front can resume.
            return resume_existing(&state, &g, &grid)
                .map(|r| (axum::http::StatusCode::OK, Json(r)));
        }
        return Err(ApiError::Conflict("already played"));
    }

    let now = Utc::now();
    let game_id = Uuid::now_v7();
    let active = games::ActiveModel {
        id: Set(game_id),
        grid_id: Set(grid.id),
        device_id: Set(device_id),
        user_id: Set(ctx.user_id),
        started_at: Set(now.into()),
        finished_at: Set(None),
        score: Set(0),
        max_score: Set(MAX_SCORE),
        mistakes: Set(0),
        solved: Set(0),
        status: Set("active".into()),
        answers: Set(serde_json::json!([])),
    };
    let _inserted = active.insert(db.as_ref()).await?;

    // Mint the play_token.
    let payload = PlayTokenPayload::new(game_id, device_id, now);
    let token = play_token::encode(&payload, state.config.play_token_hmac_key.as_bytes())
        .map_err(|e| ApiError::Internal(format!("play_token: {e}")))?;

    let public = public_grid_from_row(&grid)?;
    let resp = StartGameResponse {
        game: StartGameGame {
            id: game_id,
            started_at: now,
        },
        grid: public,
        play_token: StartGamePlayToken {
            token,
            expires_at: payload.expires_at,
        },
    };
    Ok((axum::http::StatusCode::CREATED, Json(resp)))
}

fn resume_existing(
    state: &AppState,
    g: &games::Model,
    grid: &grids::Model,
) -> ApiResult<StartGameResponse> {
    let started_at = g.started_at.with_timezone(&Utc);
    let payload = PlayTokenPayload::new(g.id, g.device_id, started_at);
    let token = play_token::encode(&payload, state.config.play_token_hmac_key.as_bytes())
        .map_err(|e| ApiError::Internal(format!("play_token: {e}")))?;
    Ok(StartGameResponse {
        game: StartGameGame {
            id: g.id,
            started_at,
        },
        grid: public_grid_from_row(grid)?,
        play_token: StartGamePlayToken {
            token,
            expires_at: payload.expires_at,
        },
    })
}

/// The serialised grid payload schema is owned by agent A. Until that is final this
/// helper extracts only the predicate labels and falls back to a 501 if the payload
/// shape isn't recognisable.
fn public_grid_from_row(row: &grids::Model) -> ApiResult<PublicGrid> {
    let payload = row.payload.clone();
    let rows = extract_predicates(&payload, "rows")?;
    let cols = extract_predicates(&payload, "cols")?;
    Ok(PublicGrid {
        id: row.id,
        domain: row.domain.clone(),
        mode: row.mode.clone(),
        publish_at: row.publish_at.with_timezone(&Utc),
        rows,
        cols,
        mistakes_allowed: MAX_MISTAKES,
    })
}

fn extract_predicates(payload: &serde_json::Value, key: &str) -> ApiResult<Vec<PredicateLabel>> {
    let arr = payload
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::Internal(format!("grid payload missing {key}")))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::Internal("predicate missing id".into()))?;
        let family = item
            .get("family")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::Internal("predicate missing family".into()))?;
        let label = item
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::Internal("predicate missing label".into()))?;
        let help = item.get("help").and_then(|v| v.as_str()).map(String::from);
        out.push(PredicateLabel {
            id: id.into(),
            family: family.into(),
            label: label.into(),
            help,
        });
    }
    Ok(out)
}

// ----- play -----

#[derive(Deserialize)]
pub struct PlayRequest {
    pub cell: Cell,
    pub answer: String,
    pub play_token: String,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct Cell {
    pub row: i32,
    pub col: i32,
}

#[derive(Serialize)]
pub struct PlayResponse {
    pub ok: bool,
    pub score_delta: i32,
    pub mistakes_left: i32,
    pub ended: bool,
}

pub async fn play(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<PlayRequest>,
) -> ApiResult<Json<PlayResponse>> {
    let device_id = ctx.device_id.ok_or(ApiError::Unauthorised)?;

    let payload = play_token::decode(
        &body.play_token,
        state.config.play_token_hmac_key.as_bytes(),
    )
    .map_err(|_| ApiError::Unauthorised)?;
    play_token::assert_matches(&payload, game_id, device_id).map_err(|_| ApiError::Unauthorised)?;

    if !(0..=2).contains(&body.cell.row) || !(0..=2).contains(&body.cell.col) {
        return Err(ApiError::BadRequest("cell out of range".into()));
    }
    if body.answer.trim().is_empty() {
        return Err(ApiError::BadRequest("empty answer".into()));
    }

    let _db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;

    Err(ApiError::NotImplemented(
        "play validation waiting for core::validator wiring against grid payload (agent-a)",
    ))
}

// ----- abandon + result -----

#[derive(Serialize)]
pub struct EndGameView {
    pub summary: GameSummary,
    pub solutions_by_cell: serde_json::Value,
}

#[derive(Serialize)]
pub struct GameSummary {
    pub game_id: Uuid,
    pub grid_id: Uuid,
    pub score: i32,
    pub max_score: i32,
    pub mistakes: i32,
    pub started_at: chrono::DateTime<Utc>,
    pub finished_at: chrono::DateTime<Utc>,
    pub solved: i32,
    pub share_string: String,
}

pub async fn abandon(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
    Extension(ctx): Extension<AuthContext>,
) -> ApiResult<Json<EndGameView>> {
    let device_id = ctx.device_id.ok_or(ApiError::Unauthorised)?;
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;
    let game = games::Entity::find_by_id(game_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("game"))?;
    if game.device_id != device_id {
        return Err(ApiError::Forbidden);
    }
    if game.status != "active" {
        return Err(ApiError::Conflict("game already finished"));
    }
    let now = Utc::now();
    let mut am: games::ActiveModel = game.into_active_model();
    am.status = Set("abandoned".into());
    am.finished_at = Set(Some(now.into()));
    let _ = am.update(db.as_ref()).await?;

    Err(ApiError::NotImplemented(
        "solutions reveal awaits core::generator payload schema (agent-a)",
    ))
}

pub async fn result(
    State(state): State<AppState>,
    Path(game_id): Path<Uuid>,
    Extension(ctx): Extension<AuthContext>,
) -> ApiResult<Json<EndGameView>> {
    let device_id = ctx.device_id.ok_or(ApiError::Unauthorised)?;
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;
    let game = games::Entity::find_by_id(game_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("game"))?;
    if game.device_id != device_id {
        return Err(ApiError::Forbidden);
    }
    if game.status == "active" {
        return Err(ApiError::Conflict("game still in progress"));
    }
    Err(ApiError::NotImplemented(
        "solutions reveal awaits core::generator payload schema (agent-a)",
    ))
}

// ----- /api/grids/{domain}/today -----

pub async fn today(
    State(state): State<AppState>,
    Path(domain): Path<String>,
) -> ApiResult<Json<PublicGrid>> {
    if !state.domains.contains_key(&domain) {
        return Err(ApiError::NotFound("domain"));
    }
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;
    let grid = grids::Entity::find()
        .filter(grids::Column::Domain.eq(domain))
        .filter(grids::Column::Mode.eq("daily"))
        .order_by_desc(grids::Column::PublishAt)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("no grid"))?;
    Ok(Json(public_grid_from_row(&grid)?))
}
