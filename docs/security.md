# kalidoku — sécurité

Synthèse des décisions et garde-fous. Source d'autorité opérationnelle : `CLAUDE.md` §4.

## Threat model rapide

| Acteur | Capacité | Mitigation |
|---|---|---|
| Joueur honnête | Joue normalement | N/A |
| Joueur tricheur (devtools) | Lire le JS, intercepter les réponses | Solutions JAMAIS envoyées avant fin partie. |
| Scraper | Frapper l'API à la chaîne | Rate limit, IDs opaques UUID, hCaptcha si pic. |
| Bot d'inscription | Spam comptes pour fausser leaderboard | Keycloak + Altcha-style PoW + email verification. |
| Attaquant CI | Compromettre une dépendance, pousser une image | Trivy CRITICAL bloque CI, Renovate auto, Snyk pré-release. |
| Attaquant infra | Pivot depuis le VPS | OpenBao secrets, ci-kalidoku scope ultra-restreint, Wazuh HIDS. |

## Couches de défense

### 1. Identité

- **Keycloak** = source de vérité (`auth.calixteair.fr`, realm `kalidoku`).
- IdP fédérés : Google + Discord + email/password local.
- Email verification obligatoire **sauf** Google (déjà vérifié par Google).
- `kalidoku.users(id, kc_sub, pseudo, created_at, role)` — mini table locale, lie au `sub` Keycloak.

### 2. Sessions

- Cookie `__Host-session` Secure HttpOnly SameSite=Strict.
- Token = 32 bytes random, **hash SHA-256 en DB**.
- TTL 30 jours, glissant : si session > 7 jours, on émet un nouveau token au prochain hit.
- Logout = `DELETE FROM sessions WHERE token_hash = ?`.

### 3. Anti-cheat

- `core/validator.rs` est la **seule** porte de validation des réponses.
- À chaque `POST /play` :
  1. Auth via cookie session.
  2. Vérifier `play_token` HMAC (signature `{game_id, device_id, started_at}`).
  3. Vérifier que la partie n'est pas finie.
  4. `core::validator::validate_answer` → `{Match, Wrong, UnknownEntity, AlreadyUsed}`.
  5. Mettre à jour `games.answers`, `games.score`, `games.mistakes`.
- Solutions complètes (`solutionsByCell`) renvoyées seulement par `/result` et `/abandon`.

### 4. Anti-bot

- **Altcha PoW invisible** sur inscription Keycloak via SPI custom (cf. wiki `[[devsecvault-altcha-spi]]`).
- **Altcha PoW** côté backend Rust également : challenge HMAC-SHA256 émis par le serveur, résolu en ~50–500 ms côté client (invisible), vérifié au prochain endpoint sensible (start game, duel create).
- **Self-hosted, RGPD friendly** : zéro service tiers, aucune télémétrie navigateur.
- **Rate limit** `tower_governor` :
  - 1 req / 250 ms par device (cookie).
  - 60 req / min par IP toutes routes confondues.
  - Burst 10, token bucket.

### 5. Anti-scraping

- IDs **UUID v7** partout, jamais d'auto-increment exposé.
- Pagination par cursor opaque (HMAC-signé) sur tous les endpoints liste.
- Liens de duel signés HMAC dans `?sig=` (déjà dans OpenAPI `/api/duels/{id}`).

### 6. Headers HTTP

```
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
Content-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'nonce-<random>'; connect-src 'self' https://hcaptcha.com; frame-src https://hcaptcha.com
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()
```

### 7. Secrets

- Tout via OpenBao (cf. `infra/bao/setup.md`).
- Charset DB password : alphanumeric uniquement (gotcha YAML/sed/`$` shell expansion — voir wiki Wazuh).
- HMAC keys : 32+ bytes random base64.
- Jamais dans logs : tronquer/hasher PII.

### 8. CI/CD

- **gitleaks** sur chaque PR + pre-commit local.
- **Trivy** scan FS sur PR + scan image après push, **CRITICAL** = échec.
- **Snyk** lancé manuellement avant chaque release `v*`.
- **Renovate** auto-merge minor/patch en green CI.
- Image runtime **distroless nonroot**, surface d'attaque minimale.

### 9. RGPD

- Pages légales `/legal/{privacy,cookies,terms}` — copie modèle DSV.
- Suppression de compte : flag `users.deleted_at` + scrub PII (`pseudo` → `deleted_<n>`, `email` → null), score conservé pour ne pas casser les leaderboards historiques.
- Export JSON sur demande via `/api/me/export`.
- Logs de production rétention 30j puis anonymisation IP.
- Bandeau cookie minimal (Umami est privacy-first, hCaptcha = stricte nécessité).

### 10. Monitoring

- Logs JSON `tracing` → stdout → Wazuh agent host.
- Alertes Wazuh (rules level ≥ 10) → mail `soc@calixteair.fr` (sidecar postfix → Mailcow).
- Métriques Prometheus optionnelles : exposer `/metrics` OAuth-protégé pour `prometheus-soc` existant.
- Uptime externe : healthchecks.io ping `/api/health` toutes les 5 min → notif Discord en cas de DOWN.

## Procédures incident

- Compromission ci-kalidoku → `userdel -r ci-kalidoku` + remplacer GitHub Secret.
- Fuite de HMAC key → rotate secret Bao + restart server (sessions invalidées).
- Cheat massif → réinitialiser leaderboard du jour, relancer la grille.
- DDoS → Cloudflare bloqué pour toi → activer challenge JS sur NPM via CrowdSec bouncer (déjà en place).
