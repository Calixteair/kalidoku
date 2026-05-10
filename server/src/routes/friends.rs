//! Friendships routes — placeholders for phase 2. The OpenAPI v1 contract does not
//! yet expose a /friends/* endpoint surface; this module exists so the friends
//! repo & service can land incrementally without router gymnastics.

use crate::error::{ApiError, ApiResult};

pub async fn placeholder() -> ApiResult<()> {
    Err(ApiError::NotImplemented(
        "friendships endpoints arrive with phase 2",
    ))
}
