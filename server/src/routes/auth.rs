//! Auth handlers — minimal MVP.
//!
//! - `GET  /api/auth/login`    : redirect to Keycloak with PKCE.
//! - `GET  /api/auth/callback` : exchange code → set session cookie.
//! - `POST /api/auth/logout`   : revoke session.
//! - `GET  /api/me`            : current user profile.
//!
//! The full code-exchange path (calling Keycloak's token endpoint, validating the
//! id_token via JWKS, upserting the local `users` row) is structured but not yet
//! wired to a real Keycloak — agent E spins up the realm separately. Until then the
//! endpoints return `501 Not Implemented` so the front can stub them safely.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthContext;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginQuery {
    pub redirect_to: Option<String>,
}

pub async fn login(
    State(_state): State<AppState>,
    Query(_q): Query<LoginQuery>,
) -> ApiResult<axum::response::Response> {
    Err(ApiError::NotImplemented(
        "OIDC PKCE redirect waiting for keycloak realm wiring (agent-c next pass)",
    ))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn callback(
    State(_state): State<AppState>,
    Query(_q): Query<CallbackQuery>,
) -> ApiResult<axum::response::Response> {
    Err(ApiError::NotImplemented(
        "OIDC token exchange waiting for keycloak realm (agent-c next pass)",
    ))
}

pub async fn logout() -> StatusCode {
    // Cookie deletion is performed in middleware-aware variant; for the MVP scaffold
    // we just answer 204 — the full revoke runs once OIDC login is live.
    StatusCode::NO_CONTENT
}

#[derive(Serialize)]
pub struct Premium {
    pub active: bool,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub struct Me {
    pub id: uuid::Uuid,
    pub pseudo: String,
    pub email: String,
    pub locale: String,
    pub premium: Premium,
    pub avatar_url: Option<String>,
}

pub async fn me(
    State(_state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> ApiResult<Json<Me>> {
    if !ctx.is_authenticated {
        return Err(ApiError::Unauthorised);
    }
    Err(ApiError::NotImplemented(
        "/me reads the users row — finalised once OIDC callback persists profile",
    ))
}
