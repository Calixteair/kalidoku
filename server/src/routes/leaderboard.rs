//! `/api/leaderboard/{domain}` — periodised, multi-metric leaderboard.
//!
//! One endpoint, three periods, five sort keys. The response always carries
//! aggregate metrics so the frontend can render the same component regardless
//! of period — a daily slice is just an aggregate over today's single game.
//!
//! Periods:
//! - `daily`: games whose grid is the current daily for the domain (the worker
//!   publishes one per day). One game per device by design, so the aggregate
//!   degenerates to a row per game.
//! - `weekly`: sliding 7 days — finished_at >= now() - 7d. Aggregates per
//!   `user_id`. Anonymous games (NULL user_id) are excluded so a meaningful
//!   pseudonym backs every row.
//! - `all-time`: no time filter. Same anonymous exclusion as weekly.
//!
//! Sort keys (default `score`):
//! - `score`   — best score across the period, ties broken by originality.
//! - `originality` — best originality.
//! - `time`    — average resolution time on wins, ascending (faster = better).
//! - `count`   — number of finished games.
//! - `wins`    — number of won games.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::entities::{games, grids, users};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

const PAGE_SIZE: usize = 50;

// ----- Request -----

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Period {
    Daily,
    Weekly,
    AllTime,
}

impl Default for Period {
    fn default() -> Self {
        Self::Daily
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    Score,
    Originality,
    Time,
    Count,
    Wins,
}

impl Default for SortKey {
    fn default() -> Self {
        Self::Score
    }
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default)]
    pub period: Period,
    #[serde(default)]
    pub sort: SortKey,
}

// ----- Response -----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProfile {
    pub id: uuid::Uuid,
    pub pseudo: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub profile: PublicProfile,
    /// Number of finished games over the period (>= 1 since rows with zero
    /// finished games never reach the page).
    pub games_count: i32,
    /// Number of games whose status ended at `won`.
    pub wins: i32,
    /// Best score in the period (single-game basis).
    pub best_score: i32,
    /// Average score over the period, rounded to the nearest unit. Daily
    /// degenerates to `best_score`.
    pub avg_score: i32,
    /// Best originality (0..=100) reached over the period.
    pub best_originality: i32,
    /// Average originality, rounded.
    pub avg_originality: i32,
    /// Average resolution time in seconds over the player's *won* games.
    /// Null when the player has zero wins — `time` sort puts them last.
    pub avg_time_seconds: Option<i32>,
    /// ISO timestamp of the most recent finished game in the period.
    pub last_finished_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardPage {
    pub period: &'static str,
    pub sort: &'static str,
    pub items: Vec<LeaderboardEntry>,
    pub next_cursor: Option<String>,
}

// ----- Handlers -----

pub async fn list(
    State(state): State<AppState>,
    Path(domain): Path<String>,
    Query(q): Query<LeaderboardQuery>,
) -> ApiResult<Json<LeaderboardPage>> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db unavailable".into()))?;
    if !state.domains.contains_key(&domain) {
        return Err(ApiError::NotFound("domain"));
    }

    // Pull every relevant game. We do the aggregation in-memory because the
    // per-period working set stays small (< 10k rows on a single domain even
    // at v1 scale) and the alternative (raw SQL via SeaORM's query builder
    // for a GROUP BY + window functions) costs more than it saves.
    let game_rows = fetch_games_for_period(db.as_ref(), &domain, q.period).await?;

    // Resolve user_id → pseudo in a single pass to dodge the N+1 the old
    // today handler had.
    let user_ids: Vec<uuid::Uuid> = game_rows.iter().filter_map(|g| g.user_id).collect();
    let pseudos = if user_ids.is_empty() {
        HashMap::new()
    } else {
        users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids))
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(|u| (u.id, u.pseudo))
            .collect::<HashMap<_, _>>()
    };

    let mut aggregates = aggregate_by_player(&game_rows, &pseudos, q.period);
    sort_aggregates(&mut aggregates, q.sort);

    let items = aggregates
        .into_iter()
        .take(PAGE_SIZE)
        .enumerate()
        .map(|(idx, mut entry)| {
            entry.rank = (idx + 1) as i32;
            entry
        })
        .collect();

    Ok(Json(LeaderboardPage {
        period: period_str(q.period),
        sort: sort_str(q.sort),
        items,
        next_cursor: None,
    }))
}

/// Back-compat alias for the legacy `GET /api/leaderboard/{domain}/today` URL.
/// Front-end was on the old shape until phase 2.5; keeping the path live until
/// it migrates to the unified handler.
pub async fn today(
    state: State<AppState>,
    domain: Path<String>,
) -> ApiResult<Json<LeaderboardPage>> {
    let q = Query(LeaderboardQuery {
        period: Period::Daily,
        sort: SortKey::Score,
    });
    list(state, domain, q).await
}

// ----- Internals -----

async fn fetch_games_for_period(
    db: &sea_orm::DatabaseConnection,
    domain: &str,
    period: Period,
) -> ApiResult<Vec<games::Model>> {
    let finished_statuses = vec!["won", "lost", "abandoned"];
    match period {
        Period::Daily => {
            // Daily = games tied to today's daily grid. Same lookup as the
            // legacy handler but with a wider 24h window to absorb worker
            // publish jitter.
            let since = Utc::now() - Duration::days(1);
            let grid = grids::Entity::find()
                .filter(grids::Column::Domain.eq(domain.to_string()))
                .filter(grids::Column::Mode.eq("daily"))
                .filter(
                    grids::Column::PublishAt
                        .gte::<chrono::DateTime<chrono::FixedOffset>>(since.into()),
                )
                .order_by_desc_col(grids::Column::PublishAt)
                .one(db)
                .await?
                .ok_or(ApiError::NotFound("no grid"))?;
            let rows = games::Entity::find()
                .filter(games::Column::GridId.eq(grid.id))
                .filter(games::Column::Status.is_in(finished_statuses))
                .all(db)
                .await?;
            Ok(rows)
        }
        Period::Weekly | Period::AllTime => {
            // Span all daily grids of the domain so we never pick up solo or
            // duel games (which use ad-hoc grids unrelated to the daily
            // ladder).
            let grid_ids: Vec<uuid::Uuid> = grids::Entity::find()
                .filter(grids::Column::Domain.eq(domain.to_string()))
                .filter(grids::Column::Mode.eq("daily"))
                .all(db)
                .await?
                .into_iter()
                .map(|g| g.id)
                .collect();
            if grid_ids.is_empty() {
                return Ok(Vec::new());
            }
            let mut q = games::Entity::find()
                .filter(games::Column::GridId.is_in(grid_ids))
                .filter(games::Column::Status.is_in(finished_statuses));
            if period == Period::Weekly {
                let since = Utc::now() - Duration::days(7);
                q = q.filter(
                    games::Column::FinishedAt
                        .gte::<chrono::DateTime<chrono::FixedOffset>>(since.into()),
                );
            }
            // Exclude anonymous rows on weekly/all-time: we won't surface them
            // and pulling them just to drop them in-memory wastes a join.
            q = q.filter(games::Column::UserId.is_not_null());
            Ok(q.all(db).await?)
        }
    }
}

fn aggregate_by_player(
    games: &[games::Model],
    pseudos: &HashMap<uuid::Uuid, String>,
    period: Period,
) -> Vec<LeaderboardEntry> {
    // For daily we keep the "one row per game" semantics: each game becomes
    // its own entry. That lets the same UI render daily and weekly without
    // branching, and players who didn't sign in still see their score.
    if period == Period::Daily {
        return games
            .iter()
            .map(|g| entry_from_single_game(g, pseudos))
            .collect();
    }

    // Weekly + all-time: group by user_id (anonymous already filtered out
    // upstream).
    let mut by_user: HashMap<uuid::Uuid, Vec<&games::Model>> = HashMap::new();
    for g in games {
        if let Some(uid) = g.user_id {
            by_user.entry(uid).or_default().push(g);
        }
    }

    by_user
        .into_iter()
        .map(|(uid, gs)| entry_from_user_games(uid, &gs, pseudos))
        .collect()
}

fn entry_from_single_game(
    g: &games::Model,
    pseudos: &HashMap<uuid::Uuid, String>,
) -> LeaderboardEntry {
    let pseudo = g
        .user_id
        .and_then(|uid| pseudos.get(&uid).cloned())
        .unwrap_or_else(|| "anonymous".into());
    let avg_time = if g.status == "won" {
        finish_time_seconds(g)
    } else {
        None
    };
    LeaderboardEntry {
        rank: 0,
        profile: PublicProfile {
            id: g.user_id.unwrap_or_else(uuid::Uuid::nil),
            pseudo,
            avatar_url: None,
        },
        games_count: 1,
        wins: i32::from(g.status == "won"),
        best_score: g.score,
        avg_score: g.score,
        best_originality: g.originality_score,
        avg_originality: g.originality_score,
        avg_time_seconds: avg_time,
        last_finished_at: g
            .finished_at
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(Utc::now),
    }
}

fn entry_from_user_games(
    uid: uuid::Uuid,
    games: &[&games::Model],
    pseudos: &HashMap<uuid::Uuid, String>,
) -> LeaderboardEntry {
    let pseudo = pseudos
        .get(&uid)
        .cloned()
        .unwrap_or_else(|| "anonymous".into());

    let count = games.len() as i32;
    let wins = games.iter().filter(|g| g.status == "won").count() as i32;
    let best_score = games.iter().map(|g| g.score).max().unwrap_or(0);
    let best_originality = games.iter().map(|g| g.originality_score).max().unwrap_or(0);

    // i64 math then i32 cast keeps the rounding correct even when count
    // * 100 doesn't fit in i32 (~21M games, unreachable but cheap to be
    // safe).
    let avg_score =
        (games.iter().map(|g| i64::from(g.score)).sum::<i64>() / i64::from(count)) as i32;
    let avg_originality = (games
        .iter()
        .map(|g| i64::from(g.originality_score))
        .sum::<i64>()
        / i64::from(count)) as i32;

    let won_times: Vec<i64> = games
        .iter()
        .filter(|g| g.status == "won")
        .filter_map(|g| finish_time_seconds(g).map(i64::from))
        .collect();
    let avg_time_seconds = if won_times.is_empty() {
        None
    } else {
        Some((won_times.iter().sum::<i64>() / won_times.len() as i64) as i32)
    };

    let last_finished_at = games
        .iter()
        .filter_map(|g| g.finished_at.map(|t| t.with_timezone(&Utc)))
        .max()
        .unwrap_or_else(Utc::now);

    LeaderboardEntry {
        rank: 0,
        profile: PublicProfile {
            id: uid,
            pseudo,
            avatar_url: None,
        },
        games_count: count,
        wins,
        best_score,
        avg_score,
        best_originality,
        avg_originality,
        avg_time_seconds,
        last_finished_at,
    }
}

fn finish_time_seconds(g: &games::Model) -> Option<i32> {
    let started = g.started_at;
    let finished = g.finished_at?;
    let secs = (finished - started).num_seconds();
    // Wall-clock anomalies (clock drift, replays) get clamped to a sane range
    // instead of leaking negative or unbounded times to the UI.
    if !(0..=86_400).contains(&secs) {
        return None;
    }
    Some(secs as i32)
}

fn sort_aggregates(items: &mut [LeaderboardEntry], sort: SortKey) {
    match sort {
        SortKey::Score => {
            // Primary: best score. Tiebreakers: originality, then most recent
            // finished_at (so a fresh PB ranks above an old one).
            items.sort_by(|a, b| {
                b.best_score
                    .cmp(&a.best_score)
                    .then_with(|| b.best_originality.cmp(&a.best_originality))
                    .then_with(|| b.last_finished_at.cmp(&a.last_finished_at))
            });
        }
        SortKey::Originality => {
            items.sort_by(|a, b| {
                b.best_originality
                    .cmp(&a.best_originality)
                    .then_with(|| b.best_score.cmp(&a.best_score))
            });
        }
        SortKey::Time => {
            // Faster = better. Players with no wins (avg_time_seconds = None)
            // sink to the bottom.
            items.sort_by(|a, b| match (a.avg_time_seconds, b.avg_time_seconds) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        SortKey::Count => {
            items.sort_by(|a, b| {
                b.games_count
                    .cmp(&a.games_count)
                    .then_with(|| b.wins.cmp(&a.wins))
            });
        }
        SortKey::Wins => {
            items.sort_by(|a, b| {
                b.wins
                    .cmp(&a.wins)
                    .then_with(|| b.best_score.cmp(&a.best_score))
            });
        }
    }
}

fn period_str(p: Period) -> &'static str {
    match p {
        Period::Daily => "daily",
        Period::Weekly => "weekly",
        Period::AllTime => "all-time",
    }
}

fn sort_str(s: SortKey) -> &'static str {
    match s {
        SortKey::Score => "score",
        SortKey::Originality => "originality",
        SortKey::Time => "time",
        SortKey::Count => "count",
        SortKey::Wins => "wins",
    }
}

// SeaORM helper for `.order_by_desc(...)` against Column variants we don't
// want to import in this file's prelude — keeps the imports tidy.
trait QueryOrderExt: sea_orm::QueryOrder + Sized {
    fn order_by_desc_col<C: ColumnTrait>(self, col: C) -> Self {
        self.order_by_desc(col)
    }
}

impl<T: sea_orm::QueryOrder> QueryOrderExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_game(
        user_id: Option<uuid::Uuid>,
        status: &str,
        score: i32,
        originality: i32,
        seconds: i64,
    ) -> games::Model {
        let started = Utc::now() - Duration::seconds(seconds);
        let finished = Utc::now();
        games::Model {
            id: uuid::Uuid::now_v7(),
            grid_id: uuid::Uuid::now_v7(),
            device_id: uuid::Uuid::now_v7(),
            user_id,
            started_at: started.with_timezone(&Utc).into(),
            finished_at: Some(finished.with_timezone(&Utc).into()),
            score,
            max_score: 900,
            mistakes: 0,
            solved: if status == "won" { 9 } else { 0 },
            status: status.into(),
            answers: serde_json::json!([]),
            originality_score: originality,
        }
    }

    #[test]
    fn finish_time_skips_negative_or_huge_durations() {
        let mut g = fake_game(None, "won", 900, 80, 120);
        // Tamper the timestamps so finish < start.
        g.started_at = Utc::now().with_timezone(&Utc).into();
        g.finished_at = Some(
            (Utc::now() - Duration::seconds(3600))
                .with_timezone(&Utc)
                .into(),
        );
        assert!(finish_time_seconds(&g).is_none());
    }

    #[test]
    fn sort_score_breaks_ties_by_originality_then_recency() {
        let u1 = uuid::Uuid::now_v7();
        let u2 = uuid::Uuid::now_v7();
        let u3 = uuid::Uuid::now_v7();
        let pseudos = [(u1, "alice"), (u2, "bob"), (u3, "carla")]
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        let games = vec![
            fake_game(Some(u1), "won", 900, 50, 120),
            fake_game(Some(u2), "won", 900, 80, 200),
            fake_game(Some(u3), "won", 800, 100, 60),
        ];
        let refs: Vec<&games::Model> = games.iter().collect();
        let by_user: HashMap<uuid::Uuid, Vec<&games::Model>> = refs
            .into_iter()
            .map(|g| (g.user_id.unwrap(), vec![g]))
            .collect();
        let mut items: Vec<LeaderboardEntry> = by_user
            .into_iter()
            .map(|(uid, gs)| entry_from_user_games(uid, &gs, &pseudos))
            .collect();
        sort_aggregates(&mut items, SortKey::Score);
        assert_eq!(items[0].profile.pseudo, "bob"); // 900 + 80 orig
        assert_eq!(items[1].profile.pseudo, "alice"); // 900 + 50 orig
        assert_eq!(items[2].profile.pseudo, "carla"); // 800
    }

    #[test]
    fn sort_time_sinks_players_with_no_wins() {
        let u1 = uuid::Uuid::now_v7();
        let u2 = uuid::Uuid::now_v7();
        let pseudos = [(u1, "winner"), (u2, "loser")]
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        let g_win = fake_game(Some(u1), "won", 900, 80, 60);
        let g_lost = fake_game(Some(u2), "lost", 200, 30, 120);
        let mut items = vec![
            entry_from_user_games(u1, &[&g_win], &pseudos),
            entry_from_user_games(u2, &[&g_lost], &pseudos),
        ];
        sort_aggregates(&mut items, SortKey::Time);
        assert_eq!(items[0].profile.pseudo, "winner");
        assert!(items[1].avg_time_seconds.is_none());
    }
}
