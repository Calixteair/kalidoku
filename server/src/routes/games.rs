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
use crate::services::solo_generator;
use crate::state::AppState;

const MAX_MISTAKES: i32 = 3;
const MAX_SCORE: i32 = 9 * 100;

// ----- start_game -----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGameRequest {
    pub domain: String,
    pub mode: String,
    #[serde(default)]
    pub duel_grid_id: Option<Uuid>,
    #[serde(default)]
    pub altcha_solution: Option<String>,
    /// Solo only. When supplied, regenerates the same grid for the same seed
    /// — used by share links and player retries. Server picks a random one if
    /// omitted.
    #[serde(default)]
    pub seed: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGameGame {
    pub id: Uuid,
    pub started_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGamePlayToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGameResponse {
    pub game: StartGameGame,
    pub grid: PublicGrid,
    pub play_token: StartGamePlayToken,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicGrid {
    pub id: Uuid,
    pub domain: String,
    pub mode: String,
    pub publish_at: chrono::DateTime<Utc>,
    pub rows: Vec<PredicateLabel>,
    pub cols: Vec<PredicateLabel>,
    pub mistakes_allowed: i32,
    /// Per-cell number of valid candidates, row-major (length 9). The list of
    /// candidate ids stays server-side; the client only sees the count so it
    /// can surface "X possible answers" without leaking the solution set.
    pub candidates_count: [u32; 9],
    /// Solo grids only — exposed so the front can build a shareable URL
    /// (`/?seed=…`). Null for daily / duel grids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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

    // Anti-bot: required only on duels for now. Anonymous daily play is allowed
    // because the rate-limit + play-token HMAC + 1-game-per-device-per-grid uniqueness
    // already cover the realistic abuse cases. The Altcha widget will be re-introduced
    // on the front when we expose duel creation; until then it would just block legit
    // anonymous players from starting their daily.
    let altcha_required = matches!(mode, GameMode::Duel);
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

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;

    // Quota check — premium short-circuits, otherwise count today's games for
    // (mode, device_or_user) and apply the free-tier limits.
    let is_prem = quota::is_premium(db.as_ref(), ctx.user_id).await?;
    let today = quota::today_count(db.as_ref(), device_id, ctx.user_id, mode).await?;
    quota::check(is_prem, mode, today).map_err(|e| match e {
        quota::QuotaError::PremiumRequired => ApiError::PaymentRequired,
        quota::QuotaError::DailyQuotaReached | quota::QuotaError::SoloQuotaReached => {
            ApiError::PaymentRequired
        }
    })?;

    // Find or create a grid for the requested mode + domain.
    // - daily: the worker publishes one per (domain, day); we just pick the
    //   most recent row.
    // - solo: generate on the fly. The seed comes from the request when the
    //   player is replaying a shared link, otherwise the server picks a fresh
    //   random one. We *always* persist the grid so play / result endpoints
    //   keep their stateless shape (game.grid_id → grids row).
    // - duel: still pending phase 2.
    let grid = match mode {
        GameMode::Daily => grids::Entity::find()
            .filter(grids::Column::Domain.eq(body.domain.clone()))
            .filter(grids::Column::Mode.eq("daily"))
            .order_by_desc(grids::Column::PublishAt)
            .one(db.as_ref())
            .await?
            .ok_or(ApiError::NotFound("no grid published yet"))?,
        GameMode::Solo => {
            generate_and_insert_solo(db.as_ref(), &state.config, &body.domain, body.seed).await?
        }
        GameMode::Duel => {
            // For duels we re-use the grid the duel pins. We do NOT verify the
            // duel signature here — anyone who reaches this code path already
            // has the grid_id, which is itself only obtainable via a valid duel
            // GET. Re-checking would force a second DB round-trip for no extra
            // security.
            let grid_id = body
                .duel_grid_id
                .ok_or_else(|| ApiError::BadRequest("duelGridId required".into()))?;
            let g = grids::Entity::find_by_id(grid_id)
                .one(db.as_ref())
                .await?
                .ok_or(ApiError::NotFound("grid"))?;
            if g.domain != body.domain {
                return Err(ApiError::BadRequest(
                    "grid does not belong to that domain".into(),
                ));
            }
            g
        }
    };

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
        originality_score: Set(0),
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

/// Generate a fresh solo grid for `domain_id`, persist it, return the row.
/// The grid is keyed by a random short seed (or the supplied one for shared
/// links), and `publish_at = now()` so the unique `(domain, mode, publish_at)`
/// index never trips even if two players replay the same seed within seconds.
pub async fn generate_and_insert_solo(
    db: &sea_orm::DatabaseConnection,
    cfg: &crate::config::AppConfig,
    domain_id: &str,
    seed: Option<u64>,
) -> ApiResult<grids::Model> {
    let root = std::path::Path::new(&cfg.domains_root);
    let solo = solo_generator::generate_solo(root, domain_id, seed)
        .await
        .map_err(|e| ApiError::Internal(format!("solo generator: {e}")))?;

    let now = Utc::now();
    let grid_id = Uuid::now_v7();
    let active = grids::ActiveModel {
        id: Set(grid_id),
        domain: Set(domain_id.to_string()),
        mode: Set("solo".to_string()),
        publish_at: Set(now.into()),
        payload: Set(solo.payload),
        // i64 cast is lossless: solo seeds are 32-bit.
        seed: Set(solo.seed as i64),
        created_at: Set(now.into()),
    };
    let inserted = active.insert(db).await?;
    Ok(inserted)
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
    // Seed is exposed to the client only for solo grids — daily grids share a
    // seed across the world and surfacing it would let a curious player
    // reverse-derive tomorrow's grid offline.
    let seed = if row.mode == "solo" {
        Some(row.seed as u64)
    } else {
        None
    };
    Ok(PublicGrid {
        id: row.id,
        domain: row.domain.clone(),
        mode: row.mode.clone(),
        publish_at: row.publish_at.with_timezone(&Utc),
        rows,
        cols,
        mistakes_allowed: MAX_MISTAKES,
        candidates_count: extract_candidates_count(&payload),
        seed,
    })
}

/// Reads `payload.candidates[row][col]` and returns the per-cell candidate
/// count in row-major order. Cells with a malformed payload default to 0 so a
/// partial dataset still surfaces a usable grid rather than 500-ing.
fn extract_candidates_count(payload: &serde_json::Value) -> [u32; 9] {
    let mut counts = [0u32; 9];
    let Some(rows) = payload.get("candidates").and_then(|v| v.as_array()) else {
        return counts;
    };
    for (r, row) in rows.iter().take(3).enumerate() {
        let Some(cols) = row.as_array() else { continue };
        for (c, cell) in cols.iter().take(3).enumerate() {
            if let Some(arr) = cell.as_array() {
                counts[r * 3 + c] = arr.len() as u32;
            }
        }
    }
    counts
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
#[serde(rename_all = "camelCase")]
pub struct PlayRequest {
    pub cell: Cell,
    pub answer: String,
    pub play_token: String,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub row: i32,
    pub col: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
    let trimmed_answer = body.answer.trim();
    if trimmed_answer.is_empty() {
        return Err(ApiError::BadRequest("empty answer".into()));
    }
    if trimmed_answer.len() > 200 {
        return Err(ApiError::BadRequest("answer too long".into()));
    }

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
    if game.mistakes >= MAX_MISTAKES {
        return Err(ApiError::Conflict("game already finished"));
    }

    let grid = grids::Entity::find_by_id(game.grid_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("grid"))?;

    // Resolve user input → entity_id via the grid's entity index. If the input
    // matches no known candidate (e.g. autocomplete proposed a station that
    // isn't part of this grid's 72 candidates), we still count it as a wrong
    // answer rather than 400-ing — the front already shows the autocomplete
    // hits, so a miss here is a real gameplay event, not a malformed request.
    let cell_idx = (body.cell.row as usize) * 3 + (body.cell.col as usize);
    let answer_norm = kalidoku_core::normalize::normalize(trimmed_answer);
    let resolved = resolve_entity_id(&grid.payload, &answer_norm);
    // Synthetic id for unresolved answers so we can keep the storage shape
    // consistent and still detect duplicate identical guesses on the same cell.
    let entity_id = resolved
        .clone()
        .unwrap_or_else(|| format!("unknown:{answer_norm}"));

    // Lock a cell only once it's been answered correctly. A wrong guess still
    // costs a mistake but doesn't freeze the cell — the player can keep trying
    // (within the global mistake budget). This matches the "every wrong guess
    // is a life lost" UX of comparable grid games.
    let answers = parse_answers(&game.answers);
    if answers.iter().any(|a| a.cell == cell_idx && a.ok) {
        return Err(ApiError::Conflict("cell already solved"));
    }
    if resolved.is_some() && answers.iter().any(|a| a.entity_id == entity_id && a.ok) {
        return Err(ApiError::Conflict("entity already used"));
    }
    // Reject the exact same guess twice in a row on the same cell so a stuck
    // player can't burn mistakes by accident-tapping the same suggestion.
    if answers
        .iter()
        .any(|a| a.cell == cell_idx && a.entity_id == entity_id && !a.ok)
    {
        return Err(ApiError::Conflict("answer already tried on this cell"));
    }

    let candidates = candidates_for_cell(&grid.payload, body.cell.row, body.cell.col)
        .ok_or_else(|| ApiError::Internal("grid payload missing candidates".into()))?;
    let ok = resolved.is_some() && candidates.iter().any(|c| c == &entity_id);

    let mut new_answers = answers;
    new_answers.push(StoredAnswer {
        cell: cell_idx,
        entity_id: entity_id.clone(),
        ok,
    });

    let now = Utc::now();
    let mut score_delta = 0_i32;
    let mut mistakes = game.mistakes;
    let mut solved = game.solved;
    let mut score = game.score;

    if ok {
        score_delta = 100;
        score += score_delta;
        solved += 1;
    } else {
        mistakes += 1;
    }
    let mistakes_left = (MAX_MISTAKES - mistakes).max(0);
    let ended = mistakes >= MAX_MISTAKES || solved >= 9;

    let answers_json = serde_json::to_value(&new_answers)
        .map_err(|e| ApiError::Internal(format!("serialise answers: {e}")))?;

    // Originality: sum (100 - fame_score) over solved cells, normalised /100.
    // We always recompute from the stored answers to stay idempotent — there's
    // never more than 9 solved cells so the cost is trivial.
    let originality = compute_originality(&grid.payload, &new_answers);

    let mut am: games::ActiveModel = game.into_active_model();
    am.score = Set(score);
    am.mistakes = Set(mistakes);
    am.solved = Set(solved);
    am.answers = Set(answers_json);
    am.originality_score = Set(i32::from(originality));
    if ended {
        am.status = Set(if solved >= 9 {
            "won".into()
        } else {
            "lost".into()
        });
        am.finished_at = Set(Some(now.into()));
    }
    am.update(db.as_ref()).await?;

    Ok(Json(PlayResponse {
        ok,
        score_delta,
        mistakes_left,
        ended,
    }))
}

/// Look up an entity_id whose canonical or alias name matches the normalised user input.
fn resolve_entity_id(payload: &serde_json::Value, normalised: &str) -> Option<String> {
    let entities = payload.get("entities")?.as_array()?;
    for ent in entities {
        let id = ent.get("id")?.as_str()?;
        let name = ent.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if kalidoku_core::normalize::normalize(name) == normalised {
            return Some(id.to_string());
        }
        if let Some(aliases) = ent.get("aliases").and_then(|v| v.as_array()) {
            for a in aliases {
                if let Some(s) = a.as_str() {
                    if kalidoku_core::normalize::normalize(s) == normalised {
                        return Some(id.to_string());
                    }
                }
            }
        }
    }
    None
}

fn candidates_for_cell(payload: &serde_json::Value, row: i32, col: i32) -> Option<Vec<String>> {
    let cells = payload.get("candidates")?.as_array()?;
    let row_arr = cells.get(row as usize)?.as_array()?;
    let cell = row_arr.get(col as usize)?.as_array()?;
    Some(
        cell.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
    )
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredAnswer {
    cell: usize,
    entity_id: String,
    ok: bool,
}

fn parse_answers(raw: &serde_json::Value) -> Vec<StoredAnswer> {
    serde_json::from_value(raw.clone()).unwrap_or_default()
}

/// Compute the originality score (0..=100) for the supplied solved answers.
/// Reads each cell's solution entity from `grid.payload.entities`. An entity
/// missing `fame_score` (or absent from the snapshot, which only happens on
/// pre-phase-2 grids generated before the field existed) is treated as fame=50,
/// neutral.
fn compute_originality(payload: &serde_json::Value, answers: &[StoredAnswer]) -> u8 {
    let entities = payload
        .get("entities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            let mut by_id = std::collections::HashMap::<&str, u8>::new();
            for ent in arr {
                let Some(id) = ent.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let fame = ent
                    .get("fame_score")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u8::try_from(v.min(100)).ok())
                    .unwrap_or(kalidoku_core::entity::FAME_NEUTRAL);
                by_id.insert(id, fame);
            }
            by_id
        })
        .unwrap_or_default();

    let raw: u32 = answers
        .iter()
        .filter(|a| a.ok)
        .map(|a| {
            let fame = entities
                .get(a.entity_id.as_str())
                .copied()
                .unwrap_or(kalidoku_core::entity::FAME_NEUTRAL);
            u32::from(100u8 - fame)
        })
        .sum();
    kalidoku_core::scoring::normalise(raw)
}

// ----- abandon + result -----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndGameView {
    pub summary: GameSummary,
    pub solutions_by_cell: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
    /// Originality score normalised to 0..=100. Reflects how niche the entities
    /// the player picked were — see `kalidoku_core::scoring`.
    pub originality_score: i32,
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
    if game.status == "active" {
        let now = Utc::now();
        let mut am: games::ActiveModel = game.clone().into_active_model();
        am.status = Set("abandoned".into());
        am.finished_at = Set(Some(now.into()));
        am.update(db.as_ref()).await?;
    }
    let game = games::Entity::find_by_id(game_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("game"))?;
    let grid = grids::Entity::find_by_id(game.grid_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("grid"))?;
    Ok(Json(end_game_view(&game, &grid)?))
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
    let grid = grids::Entity::find_by_id(game.grid_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("grid"))?;
    Ok(Json(end_game_view(&game, &grid)?))
}

/// Build the EndGameView from a finished game + its grid. The candidates per cell
/// come straight from the grid payload's `candidates` 3x3 array, mapped to the
/// `entities` index for human-readable names.
fn end_game_view(game: &games::Model, grid: &grids::Model) -> ApiResult<EndGameView> {
    let started_at = game.started_at.with_timezone(&Utc);
    let finished_at = game
        .finished_at
        .map_or(started_at, |t| t.with_timezone(&Utc));

    let stored = parse_answers(&game.answers);
    let solved = stored.iter().filter(|a| a.ok).count() as i32;
    let share_string = build_share_string(&stored);

    let solutions_by_cell = build_solutions_by_cell(&grid.payload)?;

    let summary = GameSummary {
        game_id: game.id,
        grid_id: game.grid_id,
        score: game.score,
        max_score: game.max_score,
        mistakes: game.mistakes,
        started_at,
        finished_at,
        solved,
        share_string,
        originality_score: game.originality_score,
    };
    Ok(EndGameView {
        summary,
        solutions_by_cell,
    })
}

fn build_solutions_by_cell(payload: &serde_json::Value) -> ApiResult<serde_json::Value> {
    let cells = payload
        .get("candidates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::Internal("grid payload missing candidates".into()))?;
    let entities = payload
        .get("entities")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ApiError::Internal("grid payload missing entities".into()))?;
    // Build a quick id → (name, fame_score) lookup. fame is None on entities
    // baked before phase 2 — the client renders the badge only when present.
    let mut info_by_id: std::collections::HashMap<String, (String, Option<u8>)> =
        std::collections::HashMap::new();
    for ent in entities {
        let Some(id) = ent.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let name = ent
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_string();
        let fame = ent
            .get("fame_score")
            .and_then(serde_json::Value::as_u64)
            .and_then(|v| u8::try_from(v.min(100)).ok());
        info_by_id.insert(id.to_string(), (name, fame));
    }
    let mut out = Vec::with_capacity(9);
    for (r, row_arr) in cells.iter().enumerate() {
        let row_arr = row_arr
            .as_array()
            .ok_or_else(|| ApiError::Internal("malformed candidates row".into()))?;
        for (c, cell) in row_arr.iter().enumerate() {
            let ids = cell
                .as_array()
                .ok_or_else(|| ApiError::Internal("malformed candidates cell".into()))?;
            let candidates: Vec<_> = ids
                .iter()
                .filter_map(|v| v.as_str())
                .map(|id| {
                    let (name, fame) = info_by_id
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| (id.to_string(), None));
                    let mut obj = serde_json::json!({
                        "id": id,
                        "name": name,
                    });
                    if let Some(f) = fame {
                        obj["fameScore"] = serde_json::Value::from(f);
                    }
                    obj
                })
                .collect();
            out.push(serde_json::json!({
                "cell": { "row": r, "col": c },
                "candidates": candidates,
            }));
        }
    }
    Ok(serde_json::Value::Array(out))
}

/// Wordle-style emoji string. Cells are placed by grid position; if the user never
/// played a cell we use ⬜, ✅ for a correct answer, ❌ for a wrong one.
fn build_share_string(answers: &[StoredAnswer]) -> String {
    let mut grid = ['⬜'; 9];
    for a in answers {
        if a.cell < 9 {
            grid[a.cell] = if a.ok { '✅' } else { '❌' };
        }
    }
    let mut out = String::with_capacity(20);
    for (i, c) in grid.iter().enumerate() {
        out.push(*c);
        if (i + 1) % 3 == 0 && i < 8 {
            out.push('\n');
        }
    }
    out
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
