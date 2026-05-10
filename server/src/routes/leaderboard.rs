//! `/api/leaderboard/{domain}/today`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::entities::{games, grids, users};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CursorQuery {
    pub cursor: Option<String>,
}

#[derive(Serialize)]
pub struct PublicProfile {
    pub id: uuid::Uuid,
    pub pseudo: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub profile: PublicProfile,
    pub score: i32,
    pub finished_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct LeaderboardPage {
    pub items: Vec<LeaderboardEntry>,
    pub next_cursor: Option<String>,
}

const PAGE_SIZE: u64 = 50;

pub async fn today(
    State(state): State<AppState>,
    Path(domain): Path<String>,
    Query(_q): Query<CursorQuery>,
) -> ApiResult<Json<LeaderboardPage>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;
    if !state.domains.contains_key(&domain) {
        return Err(ApiError::NotFound("domain"));
    }
    let since = Utc::now() - Duration::days(1);
    let last_grid = grids::Entity::find()
        .filter(grids::Column::Domain.eq(domain.clone()))
        .filter(grids::Column::Mode.eq("daily"))
        .filter(grids::Column::PublishAt.gte::<chrono::DateTime<chrono::FixedOffset>>(since.into()))
        .order_by_desc(grids::Column::PublishAt)
        .one(db.as_ref())
        .await?
        .ok_or(ApiError::NotFound("no grid"))?;
    let paginator = games::Entity::find()
        .filter(games::Column::GridId.eq(last_grid.id))
        .filter(games::Column::Status.is_in(vec!["won", "lost", "abandoned"]))
        .order_by_desc(games::Column::Score)
        .paginate(db.as_ref(), PAGE_SIZE);
    let page = paginator.fetch_page(0).await?;
    let mut items = Vec::with_capacity(page.len());
    let mut rank = 1;
    for g in page {
        let pseudo = if let Some(uid) = g.user_id {
            users::Entity::find_by_id(uid)
                .one(db.as_ref())
                .await?
                .map_or_else(|| "anonymous".to_string(), |u| u.pseudo)
        } else {
            "anonymous".into()
        };
        items.push(LeaderboardEntry {
            rank,
            profile: PublicProfile {
                id: g.user_id.unwrap_or_else(uuid::Uuid::nil),
                pseudo,
                avatar_url: None,
            },
            score: g.score,
            finished_at: g
                .finished_at
                .map(|t| t.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        });
        rank += 1;
    }
    Ok(Json(LeaderboardPage {
        items,
        next_cursor: None,
    }))
}
