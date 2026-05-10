# Agent C — `server/`

## Périmètre

Tout `server/` + migrations.

## Livrables

### 1. Schéma DB (migrations SeaORM)

```sql
-- 0001 — base tables
users         (id uuid pk, kc_sub text unique, pseudo citext unique, email citext, locale text, role text, created_at, deleted_at)
devices       (id uuid pk, user_id fk users null, ua text, ip_first inet, last_seen timestamptz)
sessions      (token_hash bytea pk, device_id fk, user_id fk, expires_at, created_at)

-- 0002 — domains + grids
domains       (id text pk, version text, active bool, metadata jsonb)
grids         (id uuid pk, domain text fk, mode text, publish_at timestamptz, payload jsonb, seed bigint,
               UNIQUE (domain, mode, publish_at))

-- 0003 — games + plays
games         (id uuid pk, grid_id fk, device_id fk, user_id fk null, started_at, finished_at,
               score int, max_score int, mistakes int, answers jsonb,
               UNIQUE (grid_id, device_id))

-- 0004 — friendships + duels
friendships   (user_a fk, user_b fk, status text, created_at, PRIMARY KEY (user_a, user_b))
duels         (id uuid pk, grid_id fk, owner_user_id fk, share_sig bytea, expires_at)
```

### 2. Modules

- `auth/` :
  - `oidc_login`, `oidc_callback` handlers (PKCE).
  - `validate_jwt` (JWKS Keycloak cache).
  - `create_session`, `revoke_session`.
  - Middleware `RequireAuth`.

- `domains/` :
  - In-memory cache des `Domain` chargés au boot (`Arc<HashMap<String, Domain>>`).
  - Endpoint `/api/domains` + `/api/domains/{id}/autocomplete` (utilise `core::search::search`).

- `games/` :
  - `start_game(domain, mode)` → vérif quota free tier + insert en DB + génère `play_token` HMAC.
  - `play(game_id, cell, answer, play_token)` → vérif HMAC, récup `Domain` + `Grid`, appel `core::validator::validate_answer`, update game.
  - `finish_or_abandon` → revele les solutions.

- `leaderboard/` :
  - Top 50 par `(domain, date)` paginé curseur opaque.

- `duels/` :
  - `create_duel(domain)` → génère grille via worker queue → renvoie `share_url` signé HMAC.
  - `view_duel(id, sig)` → renvoie résumé des 1-2 joueurs.

- `middleware/` :
  - `tower_governor` (rate limit).
  - `tower_http::cors` (allowlist depuis config).
  - `tower_http::set_header` (headers de sécu, voir `docs/security.md`).
  - `tower_http::trace`.

### 3. Quotas / monétisation hooks (phase 2-ready)

- Module `quota.rs` :
  ```rust
  pub fn check(user: Option<UserId>, mode: GameMode, day_count: u32) -> Result<(), QuotaError>
  ```
- Au MVP : 1 daily/jour, 3 solo/jour, pas de duel.
- Phase 2 : si `users.premium_active`, retourne toujours OK.
- Stripe Checkout endpoint réservé (`/api/billing/...`) — non implémenté MVP.

### 4. Tests

- Tests d'intégration via `axum::test::TestServer` + `sqlx::test`.
- Mock Keycloak via `wiremock`.
- Smoke test du parcours daily : start → 9 plays → result.

## Acceptance

- [ ] `cargo build -p kalidoku-server`.
- [ ] `sea-orm-cli migrate up` applique tout sans erreur.
- [ ] `curl /api/health` → 200 OK JSON.
- [ ] Parcours daily complet via tests d'intégration.
- [ ] `pnpm openapi:gen` côté web ne casse pas (= pas de drift OpenAPI).
