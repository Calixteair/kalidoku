//! Authentication building blocks: OIDC PKCE handshake, JWT validation, opaque sessions.

pub mod jwks;
pub mod jwt;
pub mod oidc;
pub mod session;

pub use session::{
    hash_session_token, mint_session_token, AuthContext, COOKIE_DEVICE, COOKIE_SESSION,
    DEVICE_COOKIE_TTL_DAYS, SESSION_TTL_DAYS,
};
