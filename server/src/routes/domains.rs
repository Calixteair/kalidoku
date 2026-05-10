//! `/api/domains` and `/api/domains/{id}/autocomplete`.
//!
//! `list` serves from the in-memory state populated at boot. `autocomplete`
//! proxies to Meilisearch through `services::meili`. The proxy stays paranoid
//! about input sizing (q ≤ 100 chars, limit ∈ [1,20]) because the path is
//! unauthenticated and hot — we do not want a runaway client to drive Meili
//! into a wide-search GC pause.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::services::meili::SearchHit;
use crate::state::AppState;

#[derive(Serialize)]
pub struct DomainSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub available_modes: Vec<String>,
}

#[derive(Deserialize)]
pub struct LocaleQuery {
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "fr".into()
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<LocaleQuery>,
) -> Json<Vec<DomainSummary>> {
    let items = state
        .domains
        .values()
        .map(|d| DomainSummary {
            id: d.id.clone(),
            name: if q.locale == "en" {
                d.name_en.clone()
            } else {
                d.name_fr.clone()
            },
            version: d.version.clone(),
            description: d.description.clone(),
            available_modes: d.available_modes.clone(),
        })
        .collect();
    Json(items)
}

/// Maximum query length we forward to Meili. 100 is generous for a station /
/// movie / actor name and well under Meili's own 1000-char ceiling.
const MAX_Q_LEN: usize = 100;
/// Hard ceiling on the `limit` query param to keep the response bounded.
const MAX_LIMIT: u32 = 20;
/// Default `limit` when the front omits it.
const DEFAULT_LIMIT: u32 = 8;

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

pub async fn autocomplete(
    State(state): State<AppState>,
    Path(domain): Path<String>,
    Query(query): Query<AutocompleteQuery>,
) -> ApiResult<Json<Vec<SearchHit>>> {
    if !state.domains.contains_key(&domain) {
        return Err(ApiError::NotFound("domain"));
    }

    // q: trim then validate. We match by character count rather than byte
    // length so multibyte Unicode (Châtelet, 神戸) doesn't blow the budget.
    let q = query.q.trim();
    let q_len = q.chars().count();
    if q_len == 0 {
        return Err(ApiError::BadRequest("q must not be empty".into()));
    }
    if q_len > MAX_Q_LEN {
        return Err(ApiError::BadRequest(format!(
            "q must be at most {MAX_Q_LEN} characters"
        )));
    }

    // limit: clamp rather than 400. Front-end pagination glitches shouldn't
    // surface as user-visible errors when the safe behaviour is "return up to
    // MAX_LIMIT".
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let meili = state
        .meili
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable("search not configured".into()))?;
    let hits = meili.search(&domain, q, limit).await?;
    Ok(Json(hits))
}
