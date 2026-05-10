//! Application error type. Maps cleanly to JSON responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorised")]
    Unauthorised,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(&'static str),
    #[error("rate limited")]
    RateLimited,
    #[error("altcha required")]
    AltchaRequired,
    #[error("payment required")]
    PaymentRequired,
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct Body {
    code: &'static str,
    message: String,
}

impl ApiError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            ApiError::NotImplemented(_) => "not_implemented",
            ApiError::NotFound(_) => "not_found",
            ApiError::BadRequest(_) => "bad_request",
            ApiError::Unauthorised => "unauthorised",
            ApiError::Forbidden => "forbidden",
            ApiError::Conflict(_) => "conflict",
            ApiError::RateLimited => "rate_limited",
            ApiError::AltchaRequired => "altcha_required",
            ApiError::PaymentRequired => "payment_required",
            ApiError::Internal(_) => "internal",
        }
    }

    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorised => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiError::AltchaRequired => StatusCode::PRECONDITION_FAILED,
            ApiError::PaymentRequired => StatusCode::PAYMENT_REQUIRED,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Body {
            code: self.code(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

impl From<sea_orm::DbErr> for ApiError {
    fn from(value: sea_orm::DbErr) -> Self {
        ApiError::Internal(format!("db: {value}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        ApiError::Internal(value.to_string())
    }
}
