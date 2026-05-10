//! `/api/domains` and `/api/domains/{id}/autocomplete`.
//!
//! For now both routes serve from the in-memory state populated at boot. The
//! autocomplete endpoint waits on agent A finishing `core::search::search` against a
//! per-domain entity catalogue (loaded from `domains/` packs by agent F).

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
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

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    8
}

#[derive(Serialize)]
pub struct AutocompleteResult {
    pub id: String,
    pub name: String,
    pub subtitle: Option<String>,
}

/// Autocomplete is gated on agent A having published a domain-loading API in `core/`.
/// Until then we 501 to keep the contract honest. The route exists so the front can
/// flip it on without redeploying once `core::search::search` is wired with real data.
pub async fn autocomplete(
    State(state): State<AppState>,
    Path(domain): Path<String>,
    Query(_query): Query<AutocompleteQuery>,
) -> ApiResult<Json<Vec<AutocompleteResult>>> {
    if !state.domains.contains_key(&domain) {
        return Err(ApiError::NotFound("domain"));
    }
    Err(ApiError::NotImplemented(
        "autocomplete waits for core/search loading domain packs",
    ))
}
