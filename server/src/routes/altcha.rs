//! `/api/altcha/challenge` — issues a fresh PoW challenge.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::altcha;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct ChallengeResponse {
    pub algorithm: &'static str,
    pub challenge: String,
    pub salt: String,
    pub signature: String,
    pub maxnumber: u64,
}

pub async fn get(State(state): State<AppState>) -> ApiResult<Json<ChallengeResponse>> {
    let key = state.config.altcha_hmac_key.as_bytes();
    if key.is_empty() {
        return Err(ApiError::Internal("altcha hmac key not configured".into()));
    }
    let ch = altcha::issue_challenge(key, altcha::DEFAULT_MAX_NUMBER)
        .map_err(|e| ApiError::Internal(format!("altcha: {e}")))?;
    Ok(Json(ChallengeResponse {
        algorithm: ch.algorithm,
        challenge: ch.challenge,
        salt: ch.salt,
        signature: ch.signature,
        maxnumber: ch.maxnumber,
    }))
}
