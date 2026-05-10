//! Altcha replay protection (Redis SETNX with 10-minute TTL).

use deadpool_redis::Pool as RedisPool;
use std::sync::Arc;

use crate::error::{ApiError, ApiResult};

const TTL_SECONDS: u64 = 600;

pub async fn guard_replay(redis: &Arc<RedisPool>, challenge: &str) -> ApiResult<()> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| ApiError::Internal(format!("redis: {e}")))?;
    let key = format!("altcha:{challenge}");
    // SET key 1 NX EX 600 — atomic insert + expiry.
    let inserted: Option<String> = redis::cmd("SET")
        .arg(&key)
        .arg(1u32)
        .arg("NX")
        .arg("EX")
        .arg(TTL_SECONDS)
        .query_async(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(format!("redis: {e}")))?;
    if inserted.is_none() {
        return Err(ApiError::AltchaRequired);
    }
    Ok(())
}
