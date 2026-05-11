//! `/api/duels` — async duels.
//!
//! Flow:
//! - `POST /api/duels {domain, gridId?}`: when `gridId` is supplied, re-use that
//!   grid (must be `solo` or `daily` so we never expose a fresh ungenerated
//!   slot). Otherwise generate a brand-new solo grid via `solo_generator`.
//!   Persist a `duels` row + return a share URL signed with HMAC over
//!   `(duel_id, grid_id)`.
//! - `GET /api/duels/{id}?sig=…`: constant-time verify the signature against
//!   the stored `share_sig`, then return the grid summary and every player's
//!   score so a freshly-arriving friend can see who they're up against
//!   before clicking "play this grid".

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::duel_sig;
use crate::entities::{duels, games, grids, users};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const DUEL_TTL_DAYS: i64 = 30;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDuelRequest {
    pub domain: String,
    /// When supplied, re-use that grid instead of generating a fresh one.
    /// Used by EndGameModal so a player can challenge a friend on the
    /// exact grid they just played.
    #[serde(default)]
    pub grid_id: Option<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDuelResponse {
    pub duel_id: Uuid,
    pub grid_id: Uuid,
    pub share_url: String,
    pub expires_at: chrono::DateTime<Utc>,
}

pub async fn create(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateDuelRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<CreateDuelResponse>)> {
    if !state.domains.contains_key(&body.domain) {
        return Err(ApiError::NotFound("domain"));
    }
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;

    // Re-use an existing grid or generate a new solo one. The reuse path locks
    // the duel to a grid the caller has already proven exists (and that the
    // server has accepted), which is fine — the value-add of a duel is "play
    // the same grid I just played", not "generate a private one".
    let grid = if let Some(gid) = body.grid_id {
        let g = grids::Entity::find_by_id(gid)
            .one(db.as_ref())
            .await?
            .ok_or(ApiError::NotFound("grid"))?;
        if g.domain != body.domain {
            return Err(ApiError::BadRequest(
                "grid does not belong to that domain".into(),
            ));
        }
        if g.mode != "solo" && g.mode != "daily" {
            return Err(ApiError::BadRequest(
                "only solo or daily grids can be duelled".into(),
            ));
        }
        g
    } else {
        crate::routes::games::generate_and_insert_solo(
            db.as_ref(),
            &state.config,
            &body.domain,
            None,
        )
        .await?
    };

    let duel_id = Uuid::now_v7();
    let now = Utc::now();
    let expires_at = now + Duration::days(DUEL_TTL_DAYS);

    let sig = duel_sig::sign(state.config.duel_hmac_key.as_bytes(), duel_id, grid.id)
        .map_err(|e| ApiError::Internal(format!("duel sign: {e}")))?;

    let am = duels::ActiveModel {
        id: Set(duel_id),
        grid_id: Set(grid.id),
        owner_user_id: Set(ctx.user_id),
        share_sig: Set(sig.clone()),
        expires_at: Set(expires_at.into()),
        created_at: Set(now.into()),
    };
    am.insert(db.as_ref()).await?;

    let sig_b64 = duel_sig::encode(&sig);
    let base = if state.config.public_base_url.is_empty() {
        String::new()
    } else {
        state.config.public_base_url.trim_end_matches('/').into()
    };
    let share_url = format!("{base}/duel/{duel_id}?sig={sig_b64}");

    Ok((
        axum::http::StatusCode::CREATED,
        Json(CreateDuelResponse {
            duel_id,
            grid_id: grid.id,
            share_url,
            expires_at,
        }),
    ))
}

#[derive(Deserialize)]
pub struct ViewQuery {
    pub sig: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuelPlayerSummary {
    pub pseudo: String,
    pub score: i32,
    pub originality_score: i32,
    pub solved: i32,
    pub finished_at: Option<chrono::DateTime<Utc>>,
    /// Status string copied from `games.status` (`won` / `lost` / `abandoned`
    /// / `active`). The frontend uses it to badge the verdict differently.
    pub status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuelView {
    pub duel_id: Uuid,
    pub grid_id: Uuid,
    pub domain: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub players: Vec<DuelPlayerSummary>,
}

pub async fn view(
    State(state): State<AppState>,
    Path(duel_id): Path<Uuid>,
    Query(q): Query<ViewQuery>,
) -> ApiResult<Json<DuelView>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;
    let duel = duels::Entity::find_by_id(duel_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("duel"))?;

    if Utc::now() > duel.expires_at.with_timezone(&Utc) {
        return Err(ApiError::Gone);
    }

    // Constant-time signature check against the stored sig — equivalent to
    // recomputing HMAC, but cheaper and ties the check to what was actually
    // persisted at create time.
    let provided = duel_sig::decode(&q.sig).map_err(|_| ApiError::Unauthorised)?;
    if !constant_time_eq::constant_time_eq(&provided, &duel.share_sig) {
        return Err(ApiError::Unauthorised);
    }

    let grid = grids::Entity::find_by_id(duel.grid_id)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("grid"))?;

    let player_games = games::Entity::find()
        .filter(games::Column::GridId.eq(duel.grid_id))
        .order_by_desc(games::Column::Score)
        .order_by_desc(games::Column::OriginalityScore)
        .all(db.as_ref())
        .await?;

    let mut players = Vec::with_capacity(player_games.len());
    for g in player_games {
        let pseudo = if let Some(uid) = g.user_id {
            users::Entity::find_by_id(uid)
                .one(db.as_ref())
                .await?
                .map_or_else(|| "anonymous".to_string(), |u| u.pseudo)
        } else {
            "anonymous".into()
        };
        players.push(DuelPlayerSummary {
            pseudo,
            score: g.score,
            originality_score: g.originality_score,
            solved: g.solved,
            finished_at: g.finished_at.map(|t| t.with_timezone(&Utc)),
            status: g.status,
        });
    }

    Ok(Json(DuelView {
        duel_id,
        grid_id: duel.grid_id,
        domain: grid.domain,
        expires_at: duel.expires_at.with_timezone(&Utc),
        players,
    }))
}
