//! Integration test for issue #26 — OIDC callback against a wiremock-backed
//! Keycloak. We boot Postgres via testcontainers, stand up wiremock with two
//! routes:
//!
//!   - `GET  /protocol/openid-connect/certs` returns a JWK set built from a
//!     freshly generated RSA key,
//!   - `POST /protocol/openid-connect/token` returns a token response whose
//!     `id_token` is signed with that RSA key.
//!
//! Then we drive `GET /api/auth/login` to seed the in-memory `oidc_states`
//! map with a real PKCE handshake, follow the redirect, and replay the
//! `state` parameter on `GET /api/auth/callback`. The expected outcome is a
//! 302 redirect to the post-login URL plus a `Set-Cookie: __Host-session`.

#![cfg(not(target_os = "windows"))]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine as _;
use http_body_util::BodyExt;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use kalidoku_server::{
    build_router_with_options, config::AppConfig, db, migrations::Migrator, state::AppState,
    RouterOptions,
};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::{RsaPrivateKey, RsaPublicKey};
use sea_orm_migration::MigratorTrait;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_KID: &str = "kalidoku-test-kid";
const TEST_REALM_PATH: &str = "/realms/kalidoku-test";

struct OidcFixture {
    private_pem: String,
    n_b64: String,
    e_b64: String,
}

fn generate_rsa_fixture() -> OidcFixture {
    let mut rng = rand::thread_rng();
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("rsa key gen");
    let pub_key = RsaPublicKey::from(&priv_key);
    let private_pem = priv_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .expect("pkcs8 encode")
        .to_string();

    // JWK n / e are big-endian, no leading zero byte, base64url no-pad.
    let n_b64 = B64.encode(pub_key.n().to_bytes_be());
    let e_b64 = B64.encode(pub_key.e().to_bytes_be());

    // Sanity check that the public key is in a usable PEM form too — keeps the
    // imports honest (otherwise rust-analyser would flag `EncodeRsaPublicKey` as
    // unused even though it's needed at runtime when the PEM path is taken).
    let _ = pub_key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .expect("pkcs1 pem");

    OidcFixture {
        private_pem,
        n_b64,
        e_b64,
    }
}

fn jwks_body(fixture: &OidcFixture) -> Value {
    json!({
        "keys": [{
            "kid": TEST_KID,
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "n": fixture.n_b64,
            "e": fixture.e_b64,
        }]
    })
}

fn sign_id_token(fixture: &OidcFixture, issuer: &str, audience: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let claims = json!({
        "sub": "kc-sub-demo",
        "iss": issuer,
        "aud": audience,
        "exp": now + 600,
        "iat": now,
        "preferred_username": "alice",
        "email": "alice@example.com",
        "locale": "fr",
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(TEST_KID.to_string());
    let key =
        EncodingKey::from_rsa_pem(fixture.private_pem.as_bytes()).expect("encoding key from pem");
    jsonwebtoken::encode(&header, &claims, &key).expect("sign id_token")
}

#[tokio::test]
#[ignore = "spawns a Postgres container + a wiremock; opt in with `cargo test -- --ignored`"]
async fn oidc_callback_sets_session_cookie_and_302s() {
    // ---- infra: postgres + wiremock ----
    let pg = PgImage::default()
        .start()
        .await
        .expect("postgres container starts");
    let host = pg.get_host().await.expect("pg host");
    let port = pg.get_host_port_ipv4(5432).await.expect("pg port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let conn = db::connect(&url).await.expect("db pool");
    Migrator::up(&conn, None).await.expect("migrations apply");

    let mock = MockServer::start().await;
    let issuer = format!("{}{}", mock.uri(), TEST_REALM_PATH);
    let audience = "kalidoku-test-client".to_string();

    let fixture = generate_rsa_fixture();
    let id_token = sign_id_token(&fixture, &issuer, &audience);

    // /protocol/openid-connect/certs — JWKS
    Mock::given(method("GET"))
        .and(path(format!(
            "{TEST_REALM_PATH}/protocol/openid-connect/certs"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(jwks_body(&fixture)))
        .mount(&mock)
        .await;
    // /protocol/openid-connect/token — code → id_token
    Mock::given(method("POST"))
        .and(path(format!(
            "{TEST_REALM_PATH}/protocol/openid-connect/token"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id_token": id_token,
            "access_token": "fake-access-token",
            "token_type": "Bearer",
            "expires_in": 600,
        })))
        .mount(&mock)
        .await;

    // ---- app under test: real Jwks pointing at the wiremock issuer ----
    let mut cfg = AppConfig::test_fixture();
    cfg.keycloak_issuer_url = issuer.clone();
    cfg.keycloak_client_id = audience.clone();
    cfg.keycloak_client_secret = "test-secret".into();
    cfg.keycloak_redirect_url = "http://127.0.0.1/api/auth/callback".into();
    let state = AppState::new(cfg).with_db(conn);
    let app = build_router_with_options(state, RouterOptions { rate_limit: false });

    // ---- 1. /auth/login seeds the OIDC state map ----
    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_resp.status(), StatusCode::SEE_OTHER);
    let location = login_resp
        .headers()
        .get(header::LOCATION)
        .expect("login should redirect")
        .to_str()
        .unwrap()
        .to_string();
    let oidc_state = extract_query_param(&location, "state").expect("state in redirect");

    // ---- 2. /auth/callback exchanges the code + sets the session cookie ----
    let callback_uri = format!("/api/auth/callback?code=test-code&state={oidc_state}");
    let cb_resp = app
        .oneshot(
            Request::builder()
                .uri(callback_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = cb_resp.status();
    // Pull headers out before we consume the body, since `set-cookie` and `location`
    // are what we actually want to assert on a happy-path 3xx.
    let (parts, body) = cb_resp.into_parts();
    let body_bytes = body.collect().await.unwrap().to_bytes();

    assert!(
        status.is_redirection(),
        "callback expected 3xx, got {status}: {}",
        String::from_utf8_lossy(&body_bytes)
    );

    let session_cookie = parts
        .headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|s| s.contains("__Host-session="))
        .expect("callback must set the __Host-session cookie");
    assert!(
        session_cookie.contains("HttpOnly")
            && session_cookie.contains("Secure")
            && session_cookie.contains("SameSite=Strict"),
        "session cookie must carry the secure attribute set, got: {session_cookie}"
    );
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some(rest) = pair.strip_prefix(&format!("{key}=")) {
            return Some(urlencoding::decode(rest).ok()?.into_owned());
        }
    }
    None
}
