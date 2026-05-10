//! Thin client around Meilisearch v1.x search endpoint.
//!
//! Acts as a typed proxy so handlers stay agnostic of the search engine. We
//! intentionally keep the surface minimal (single `search` call) — re-keyed,
//! scoped API tokens and faceted queries are tracked for v2.
//!
//! Auth uses the master key as a Bearer token. Production must rotate this to
//! a per-index search key once Bao provisions it; the contract on this client
//! stays the same.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::ApiError;

/// Hard ceiling on the Meili response body we'll keep around when surfacing an
/// upstream error. Anything longer is truncated — Meili error payloads are tiny
/// JSON blobs in practice but a misbehaving proxy could stuff us with HTML.
const ERROR_BODY_TRUNCATE_BYTES: usize = 512;

/// Per-request timeout for the search call. The handler is on the user's hot
/// path so we'd rather 503 quickly than hang the page.
const SEARCH_TIMEOUT: Duration = Duration::from_secs(2);

/// One autocomplete hit, shaped for the front. The `camelCase` rename is a
/// no-op on these single-token field names today but kept in sync with the
/// rest of the public API so adding fields stays safe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub name: String,
}

/// Typed shape of the Meili `/indexes/{idx}/search` response — only the fields
/// we use. Other keys (`processingTimeMs`, `query`, `estimatedTotalHits`, …)
/// are intentionally ignored.
#[derive(Debug, Deserialize)]
struct MeiliSearchResponse {
    hits: Vec<SearchHit>,
}

/// Specific failure modes the handler needs to discriminate to pick a status
/// code. Kept inside the service layer; everything funnels into `ApiError`
/// at the call boundary via the `From` impl below.
#[derive(Debug)]
pub enum SearchError {
    /// Couldn't reach Meili at all (timeout, DNS, connection refused). The
    /// route maps this to `503 Service Unavailable`.
    Unavailable(String),
    /// Reached Meili but it answered non-2xx. Mapped to `502 Bad Gateway` so
    /// we can distinguish "search broken upstream" from "search not configured".
    BadUpstream { status: u16, body: String },
    /// Got 2xx but couldn't parse the body. Treated as bad gateway too.
    Decode(String),
}

impl From<SearchError> for ApiError {
    fn from(value: SearchError) -> Self {
        match value {
            SearchError::Unavailable(msg) => {
                ApiError::ServiceUnavailable(format!("search service: {msg}"))
            }
            SearchError::BadUpstream { status, body } => {
                ApiError::BadGateway(format!("search service: status={status} body={body}"))
            }
            SearchError::Decode(msg) => {
                ApiError::BadGateway(format!("search service: decode {msg}"))
            }
        }
    }
}

/// Meilisearch HTTP client. Cheap to clone via `Arc`; instantiate once at boot
/// and store as `Option<Arc<MeiliClient>>` in `AppState`.
#[derive(Debug, Clone)]
pub struct MeiliClient {
    pub http: reqwest::Client,
    pub base_url: String,
    pub key: String,
}

impl MeiliClient {
    /// Build a client. Trailing slash on `base_url` is trimmed defensively
    /// (operators tend to copy-paste it from Meili docs).
    #[must_use]
    pub fn new(base_url: String, key: String) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .timeout(SEARCH_TIMEOUT)
            .build()
            .expect("static reqwest client config");
        Arc::new(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            key,
        })
    }

    /// Run a `q`-string search against `index`, returning at most `limit` hits.
    ///
    /// Errors:
    /// - connection failure / timeout → `SearchError::Unavailable` (handler → 503),
    /// - non-2xx response             → `SearchError::BadUpstream`  (handler → 502),
    /// - 2xx but unparseable body     → `SearchError::Decode`       (handler → 502).
    pub async fn search(
        &self,
        index: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let url = format!("{}/indexes/{}/search", self.base_url, index);
        let body = serde_json::json!({
            "q": query,
            "limit": limit,
            "attributesToRetrieve": ["id", "name"],
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!(error = %e, %url, "meili search request failed");
                SearchError::Unavailable(e.to_string())
            })?;

        let status = resp.status();
        if !status.is_success() {
            let raw = resp.text().await.unwrap_or_default();
            let snippet: String = raw.chars().take(ERROR_BODY_TRUNCATE_BYTES).collect();
            warn!(%status, body = %snippet, %url, "meili search returned non-2xx");
            return Err(SearchError::BadUpstream {
                status: status.as_u16(),
                body: snippet,
            });
        }

        let parsed: MeiliSearchResponse = resp
            .json()
            .await
            .map_err(|e| SearchError::Decode(e.to_string()))?;
        Ok(parsed.hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_hit_round_trips() {
        let h = SearchHit {
            id: "metro:1".into(),
            name: "Châtelet".into(),
        };
        let s = serde_json::to_string(&h).unwrap();
        assert!(s.contains("\"id\":\"metro:1\""));
        assert!(s.contains("\"name\":\"Châtelet\""));
        let back: SearchHit = serde_json::from_str(&s).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn new_strips_trailing_slash_in_base_url() {
        let c = MeiliClient::new("http://search:7700/".into(), "k".into());
        assert_eq!(c.base_url, "http://search:7700");
    }

    #[test]
    fn unavailable_maps_to_service_unavailable() {
        let api: ApiError = SearchError::Unavailable("dns".into()).into();
        assert_eq!(api.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn bad_upstream_maps_to_bad_gateway() {
        let api: ApiError = SearchError::BadUpstream {
            status: 500,
            body: "boom".into(),
        }
        .into();
        assert_eq!(api.status(), axum::http::StatusCode::BAD_GATEWAY);
    }
}
