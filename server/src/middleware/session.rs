//! Session-resolving middleware. Looks up the `__Host-session` cookie, hashes it,
//! finds the matching row in `sessions` and exposes an [`AuthContext`] in the request
//! extensions. Anonymous traffic still gets a `kd_device` cookie so we can scope
//! free-tier quotas to the device even before login.
//!
//! The middleware mutates the response by attaching a fresh `kd_device` cookie when
//! it had to mint one. The DB write is best-effort — we log and continue if it fails
//! so that a transient DB outage doesn't block reads.

use std::str::FromStr;

use axum::{
    extract::{Request, State},
    http::{header::HeaderValue, HeaderMap, HeaderName},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use tracing::warn;
use uuid::Uuid;

use crate::auth::{hash_session_token, AuthContext, COOKIE_DEVICE, COOKIE_SESSION};
use crate::entities::{devices, sessions};
use crate::state::AppState;

fn device_cookie(value: String) -> Cookie<'static> {
    let mut c = Cookie::new(COOKIE_DEVICE, value);
    c.set_path("/");
    c.set_http_only(true);
    c.set_same_site(SameSite::Lax);
    // TODO(agent-c): set Max-Age once the `cookie::time::Duration` dependency is
    // wired through `axum-extra` re-exports — see `docs/agents/agent-c-server.md`
    // §8 follow-up. Without it the cookie is session-scoped (cleared on browser quit).
    c
}

/// Extracts the `__Host-session` value from the inbound cookie header (if any).
fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for piece in raw.split(';') {
        let piece = piece.trim();
        if let Some(rest) = piece.strip_prefix(&format!("{name}=")) {
            return Some(rest);
        }
    }
    None
}

pub async fn middleware(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    // Resolve / mint device id.
    let device_id = read_cookie(req.headers(), COOKIE_DEVICE).and_then(|v| Uuid::from_str(v).ok());
    let (device_id, set_device_cookie) = match device_id {
        Some(id) => (id, false),
        None => (Uuid::now_v7(), true),
    };

    if set_device_cookie {
        if let Some(db) = state.db.as_ref() {
            let now = Utc::now();
            let am = devices::ActiveModel {
                id: Set(device_id),
                user_id: Set(None),
                ua: Set(None),
                ip_first: Set(None),
                last_seen: Set(now.into()),
                created_at: Set(now.into()),
            };
            if let Err(e) = am.insert(db.as_ref()).await {
                warn!(error = %e, "could not persist new device");
            }
        }
    }

    // Resolve session if present.
    let mut ctx = AuthContext {
        device_id: Some(device_id),
        ..AuthContext::anonymous()
    };
    if let Some(token) = read_cookie(req.headers(), COOKIE_SESSION) {
        if let Some(db) = state.db.as_ref() {
            use sea_orm::EntityTrait;
            let hash = hash_session_token(token);
            match sessions::Entity::find_by_id(hash).one(db.as_ref()).await {
                Ok(Some(s))
                    if s.expires_at > chrono::DateTime::<chrono::FixedOffset>::from(Utc::now()) =>
                {
                    ctx.user_id = s.user_id;
                    ctx.is_authenticated = s.user_id.is_some();
                }
                Ok(_) => {}
                Err(e) => warn!(error = %e, "session lookup failed"),
            }
        }
    }

    req.extensions_mut().insert(ctx);
    let mut response = next.run(req).await;

    if set_device_cookie {
        let cookie = device_cookie(device_id.to_string()).to_string();
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response
                .headers_mut()
                .append(HeaderName::from_static("set-cookie"), value);
        }
    }

    response
}

/// Build a `kd_device` cookie value (used by handlers that need to attach it on
/// their own — e.g. session login).
#[must_use]
pub fn build_device_cookie(value: String) -> Cookie<'static> {
    device_cookie(value)
}

/// Build the secure `__Host-session` cookie matching the policy in `docs/security.md` §2.
/// Max-Age is set in the route handler that owns the response (see TODO above).
#[must_use]
pub fn build_session_cookie(token: String) -> Cookie<'static> {
    let mut c = Cookie::new(COOKIE_SESSION, token);
    c.set_path("/");
    c.set_http_only(true);
    c.set_secure(true);
    c.set_same_site(SameSite::Strict);
    c
}

/// Used by `CookieJar`-aware handlers — exposes the cookie key constants without the
/// caller importing `crate::auth`.
pub use crate::auth::{COOKIE_DEVICE as DEVICE_COOKIE_NAME, COOKIE_SESSION as SESSION_COOKIE_NAME};

/// Re-export so handlers don't have to import `axum_extra` directly.
pub type Jar = CookieJar;
