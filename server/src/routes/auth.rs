//! Auth handlers — OIDC PKCE flow against Keycloak.
//!
//! - `GET  /api/auth/login`    : redirect to Keycloak with PKCE.
//! - `GET  /api/auth/callback` : exchange code → upsert user → set session cookie.
//! - `POST /api/auth/logout`   : revoke session row + clear cookie.
//! - `GET  /api/me`            : current user profile.
//!
//! State (PKCE verifier + nonce) is held in `AppState.oidc_states` (in-memory map keyed
//! by the random `state` parameter). For a single-instance MVP that's fine; a multi-node
//! deployment should move this to Redis.

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Extension, Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration as ChDuration, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::jwt::IdTokenClaims;
use crate::auth::oidc::OidcState;
use crate::auth::session::{
    hash_session_token, mint_session_token, COOKIE_SESSION, SESSION_TTL_DAYS,
};
use crate::auth::AuthContext;
use crate::entities::{devices, sessions, users};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ---------------------------------------------------------------- /auth/login

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginQuery {
    pub redirect_to: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    Query(q): Query<LoginQuery>,
) -> ApiResult<Response> {
    let redirect_to = sanitise_redirect(q.redirect_to.as_deref());
    let oidc = OidcState::fresh(redirect_to);

    state
        .oidc_states
        .write()
        .await
        .insert(oidc.state.clone(), oidc.clone());

    let auth_endpoint = format!(
        "{}/protocol/openid-connect/auth",
        state.config.keycloak_issuer_url.trim_end_matches('/')
    );
    let url = format!(
        "{auth_endpoint}\
         ?response_type=code\
         &client_id={client}\
         &redirect_uri={redirect}\
         &scope=openid+email+profile\
         &state={oidc_state}\
         &nonce={nonce}\
         &code_challenge={challenge}\
         &code_challenge_method=S256",
        client = urlencoding::encode(&state.config.keycloak_client_id),
        redirect = urlencoding::encode(&state.config.keycloak_redirect_url),
        oidc_state = urlencoding::encode(&oidc.state),
        nonce = urlencoding::encode(&oidc.nonce),
        challenge = urlencoding::encode(&oidc.pkce.challenge),
    );

    Ok(Redirect::to(&url).into_response())
}

fn sanitise_redirect(input: Option<&str>) -> String {
    // Only allow same-origin paths to prevent open redirect.
    match input {
        Some(p) if p.starts_with('/') && !p.starts_with("//") => p.to_string(),
        _ => "/".to_string(),
    }
}

// ------------------------------------------------------------- /auth/callback

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    access_token: Option<String>,
}

pub async fn callback(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Query(q): Query<CallbackQuery>,
) -> ApiResult<Response> {
    let oidc = state.oidc_states.write().await.remove(&q.state);
    let Some(oidc) = oidc else {
        return Err(ApiError::BadRequest("unknown OIDC state".into()));
    };

    // Exchange code for tokens.
    let token_endpoint = format!(
        "{}/protocol/openid-connect/token",
        state.config.keycloak_issuer_url.trim_end_matches('/')
    );
    let resp = reqwest::Client::new()
        .post(&token_endpoint)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", &state.config.keycloak_client_id),
            ("client_secret", &state.config.keycloak_client_secret),
            ("code", &q.code),
            ("redirect_uri", &state.config.keycloak_redirect_url),
            ("code_verifier", &oidc.pkce.verifier),
        ])
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("token exchange: {e}")))?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError::Internal(format!("token endpoint: {body}")));
    }
    let tokens: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("token parse: {e}")))?;

    // Validate id_token via JWKS.
    let claims: IdTokenClaims = state
        .jwks
        .as_ref()
        .ok_or_else(|| ApiError::Internal("jwks not configured".into()))?
        .validate(&tokens.id_token)
        .await
        .map_err(|_| ApiError::Unauthorised)?;

    // Upsert user, mint session, set cookie.
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db not configured".into()))?;
    let user_id = upsert_user(db.as_ref(), &claims).await?;
    let (token, token_hash) = mint_session_token();
    let now = Utc::now();
    let expires_at = now + ChDuration::days(SESSION_TTL_DAYS);

    // Reuse the anonymous device id minted by the session middleware. If the
    // middleware couldn't persist it (DB transient failure) we mint a fresh one
    // and insert the row here so the FK to `devices` is always satisfied.
    let device_id = match ctx.device_id {
        Some(id) => id,
        None => {
            let id = Uuid::now_v7();
            let am = devices::ActiveModel {
                id: Set(id),
                user_id: Set(None),
                ua: Set(None),
                ip_first: Set(None),
                last_seen: Set(now.into()),
                created_at: Set(now.into()),
            };
            am.insert(db.as_ref())
                .await
                .map_err(|e| ApiError::Internal(format!("device insert: {e}")))?;
            id
        }
    };
    let session_row = sessions::ActiveModel {
        token_hash: Set(token_hash),
        user_id: Set(Some(user_id)),
        device_id: Set(device_id),
        created_at: Set(now.into()),
        expires_at: Set(expires_at.into()),
    };
    session_row
        .insert(db.as_ref())
        .await
        .map_err(|e| ApiError::Internal(format!("session insert: {e}")))?;

    let mut cookie = Cookie::new(COOKIE_SESSION.to_string(), token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    // Lax (not Strict): the OIDC callback is a cross-site redirect from Keycloak,
    // and Strict would prevent the freshly-set cookie from being sent on the
    // first request to the home page after redirect, leaving the user unauth'd.
    // CSRF on state-changing endpoints stays covered by `__Host-` + Secure +
    // HttpOnly + the play-token HMAC bound to the device id.
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(cookie::time::Duration::days(SESSION_TTL_DAYS));

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie.to_string().parse().expect("cookie header value"),
    );

    Ok((headers, Redirect::to(&oidc.redirect_to)).into_response())
}

async fn upsert_user(db: &sea_orm::DatabaseConnection, claims: &IdTokenClaims) -> ApiResult<Uuid> {
    if let Some(existing) = users::Entity::find()
        .filter(users::Column::KcSub.eq(&claims.sub))
        .one(db)
        .await
        .map_err(|e| ApiError::Internal(format!("user lookup: {e}")))?
    {
        return Ok(existing.id);
    }

    let id = Uuid::now_v7();
    let pseudo = claims
        .preferred_username
        .clone()
        .unwrap_or_else(|| format!("user_{}", &claims.sub[..8.min(claims.sub.len())]));
    let now = Utc::now();
    let am = users::ActiveModel {
        id: Set(id),
        kc_sub: Set(claims.sub.clone()),
        pseudo: Set(pseudo),
        email: Set(claims.email.clone()),
        locale: Set(claims.locale.clone().unwrap_or_else(|| "fr".to_string())),
        role: Set("user".to_string()),
        premium_active: Set(false),
        created_at: Set(now.into()),
        deleted_at: Set(None),
    };
    am.insert(db)
        .await
        .map_err(|e| ApiError::Internal(format!("user insert: {e}")))?;
    Ok(id)
}

// --------------------------------------------------------------- /auth/logout

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = read_session_cookie(&headers) {
        if let Some(db) = state.db.as_ref() {
            let hash = hash_session_token(token);
            let _ = sessions::Entity::delete_by_id(hash).exec(db.as_ref()).await;
        }
    }
    let mut clear = Cookie::new(COOKIE_SESSION.to_string(), String::new());
    clear.set_path("/");
    clear.set_http_only(true);
    clear.set_secure(true);
    clear.set_same_site(SameSite::Lax);
    clear.set_max_age(cookie::time::Duration::seconds(0));
    let mut hdrs = HeaderMap::new();
    hdrs.insert(
        header::SET_COOKIE,
        clear.to_string().parse().expect("cookie header value"),
    );
    Ok((StatusCode::NO_CONTENT, hdrs).into_response())
}

fn read_session_cookie(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for piece in raw.split(';') {
        let piece = piece.trim();
        if let Some(rest) = piece.strip_prefix(&format!("{COOKIE_SESSION}=")) {
            return Some(rest);
        }
    }
    None
}

// ------------------------------------------------------------------ /me

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Premium {
    pub active: bool,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: uuid::Uuid,
    pub pseudo: String,
    pub email: Option<String>,
    pub locale: String,
    pub premium: Premium,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

pub async fn me(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> ApiResult<Json<Me>> {
    let user_id = ctx.user_id.ok_or(ApiError::Unauthorised)?;
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::Internal("db not configured".into()))?;
    let user = users::Entity::find_by_id(user_id)
        .one(db.as_ref())
        .await
        .map_err(|e| ApiError::Internal(format!("user lookup: {e}")))?
        .ok_or(ApiError::Unauthorised)?;

    Ok(Json(Me {
        id: user.id,
        pseudo: user.pseudo,
        email: user.email,
        locale: user.locale,
        premium: Premium {
            active: user.premium_active,
            until: None,
        },
        avatar_url: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::sanitise_redirect;

    #[test]
    fn redirects_to_root_when_input_is_external() {
        assert_eq!(sanitise_redirect(Some("https://evil.example/")), "/");
        assert_eq!(sanitise_redirect(Some("//evil.example/")), "/");
        assert_eq!(sanitise_redirect(None), "/");
    }

    #[test]
    fn keeps_same_origin_path() {
        assert_eq!(sanitise_redirect(Some("/profile")), "/profile");
        assert_eq!(sanitise_redirect(Some("/")), "/");
    }
}
