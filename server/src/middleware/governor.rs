//! Rate limit knobs. The actual `GovernorLayer` is wired in `lib.rs::build_router`
//! because its type is generic over a `RateLimitingMiddleware` we don't depend on
//! directly. Tuning is centralised here so we have one place to bump the values.

use std::time::Duration;

/// Burst budget for the per-IP layer.
pub const IP_BURST: u32 = 10;
/// Replenish rate for the per-IP layer.
pub const IP_PERIOD: Duration = Duration::from_secs(1);

/// Burst budget for the per-device tightening.
pub const DEVICE_BURST: u32 = 4;
/// Replenish rate for the per-device tightening.
pub const DEVICE_PERIOD: Duration = Duration::from_millis(250);
