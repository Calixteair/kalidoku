//! Security header constants. The actual `SetResponseHeaderLayer` calls live in
//! `lib.rs::build_router` (each layer has its own type so we can't return them
//! from a single helper without erasing through `tower::ServiceBuilder`).

pub const HSTS: &str = "max-age=63072000; includeSubDomains; preload";
pub const REFERRER: &str = "strict-origin-when-cross-origin";
pub const X_CONTENT_TYPE: &str = "nosniff";
pub const X_FRAME: &str = "DENY";
pub const PERMISSIONS_POLICY: &str = "accelerometer=(), camera=(), geolocation=(), \
gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()";

pub const CSP: &str = "default-src 'self'; \
img-src 'self' data:; \
style-src 'self' 'unsafe-inline'; \
script-src 'self'; \
connect-src 'self'; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'self'";
