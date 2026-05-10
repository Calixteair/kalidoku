# Altcha — anti-bot PoW invisible (self-hosted)

État : ✅ secret HMAC en place dans Bao (`secret/kalidoku/prod/anti-bot.ALTCHA_HMAC_KEY`).
Reste à implémenter le challenge/verify côté backend (agent C) et le widget invisible côté front (agent D).

## Pourquoi Altcha plutôt que hCaptcha / Cloudflare Turnstile

| Critère | Altcha self-hosted | hCaptcha | Cloudflare Turnstile |
|---|---|---|---|
| Service tiers | aucun | oui | oui (Cloudflare) |
| Données navigateur transmises | aucune | fingerprinting | empreinte légère |
| RGPD bandeau cookie | non requis | recommandé | non requis |
| Coût | 0 | gratuit jusqu'à un seuil | gratuit |
| Tolérance bot avancé | moyenne | bonne | très bonne |
| Invisible | oui | possible | toujours |
| Disponibilité Cloudflare | n/a | n/a | bloqué chez nous (pas Cloudflare proxy) |

**Choix** : Altcha self-hosted en mode **invisible PoW**. Zéro tiers, zéro RGPD à gérer, suffisant pour le seuil de patience d'un cheater amateur.

## Architecture

```
                         ┌─────────────────┐
                         │  GET /api/altcha/challenge
                         │  → { algorithm, challenge, salt, signature, maxnumber }
                         │  signature = HMAC_SHA256(KEY, challenge)
                         │  challenge = SHA256(salt + secret_int)
                         └────────────────┬──────────┘
                                          │
              ┌───────────────────────────▼──────────────────┐
              │ Front: <altcha-widget> (auto / hidden)       │
              │  - brute-force secret_int in 0..maxnumber    │
              │  - find n such that SHA256(salt + n) = challenge
              │  - typical 50–500 ms CPU                     │
              └────────────────┬─────────────────────────────┘
                               │
                               ▼
                  POST /api/games  { altchaSolution: "<base64 JSON>" }
                  POST /api/duels  { altchaSolution: "<base64 JSON>" }
                  POST /api/auth/register (Keycloak SPI)
                               │
                               ▼
              ┌───────────────────────────────────────────────┐
              │ Backend verify:                               │
              │  1. base64 decode payload                     │
              │  2. HMAC-verify signature with ALTCHA_HMAC_KEY│
              │  3. compute SHA256(salt+number) → ==challenge │
              │  4. ensure not replayed (Redis SETNX 10 min)  │
              └───────────────────────────────────────────────┘
```

## Difficulty tuning

`maxnumber` contrôle le coût :

| maxnumber | client time (modern phone) | bot cost | quand l'utiliser |
|---|---|---|---|
| 50_000 | <50 ms | trivial | usage normal, latence imperceptible |
| 100_000 | ~100 ms | léger | défaut MVP |
| 500_000 | ~500 ms | sensible | sur signal de pic |
| 5_000_000 | ~5 s | dissuasif | en escalade après détection abus |

L'idée n'est pas d'arrêter un attaquant motivé — c'est de rendre **un farm massif** linéairement coûteux.

## Implémentation Rust (server/src/altcha.rs — agent C)

```rust
use hmac::{Hmac, Mac};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_MAX: u64 = 100_000;

#[derive(Serialize)]
pub struct Challenge {
    pub algorithm: &'static str,
    pub challenge: String,   // hex SHA256
    pub salt:      String,   // hex 16 bytes random
    pub signature: String,   // hex HMAC256(challenge)
    pub maxnumber: u64,
}

pub fn issue_challenge(hmac_key: &[u8], maxnumber: u64) -> Challenge {
    let mut rng = thread_rng();
    let mut salt_bytes = [0u8; 16];
    rng.fill(&mut salt_bytes);
    let salt = hex::encode(salt_bytes);
    let secret_number: u64 = rng.gen_range(0..maxnumber);

    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(secret_number.to_string().as_bytes());
    let challenge = hex::encode(hasher.finalize());

    let mut mac = HmacSha256::new_from_slice(hmac_key).expect("HMAC key");
    mac.update(challenge.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    Challenge { algorithm: "SHA-256", challenge, salt, signature, maxnumber }
}

#[derive(Deserialize)]
pub struct Solution {
    pub algorithm: String,
    pub challenge: String,
    pub salt:      String,
    pub signature: String,
    pub number:    u64,
}

pub fn verify_solution(solution: &Solution, hmac_key: &[u8]) -> bool {
    if solution.algorithm != "SHA-256" { return false; }
    let mut mac = HmacSha256::new_from_slice(hmac_key).expect("HMAC key");
    mac.update(solution.challenge.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    if !constant_time_eq::constant_time_eq(expected.as_bytes(), solution.signature.as_bytes()) {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(solution.salt.as_bytes());
    hasher.update(solution.number.to_string().as_bytes());
    let computed = hex::encode(hasher.finalize());
    constant_time_eq::constant_time_eq(computed.as_bytes(), solution.challenge.as_bytes())
}
```

Le replay-check (Redis SETNX `altcha:{challenge}` TTL 10 min) est dans le service appelant, pas dans le module pur.

## Côté front (agent D)

Le widget officiel Altcha est ~6 KB :

```html
<altcha-widget
  challengeurl="/api/altcha/challenge"
  hidefooter
  hidelogo
  auto="onload"
  name="altchaSolution">
</altcha-widget>
```

Charger avec :

```ts
import "altcha";  // npm i altcha
```

Le widget injecte `altchaSolution` (champ caché) dans le formulaire. On le lit via JS et on l'envoie dans le POST JSON kalidoku.

## Activation par endpoint

Au MVP, **on n'exige Altcha que si une heuristique le déclenche** :

- `POST /api/games` → exiger seulement si l'IP a >5 req/min OU si user anonyme.
- `POST /api/duels` → toujours exiger.
- `POST /api/auth/register` (Keycloak) → SPI Altcha (cf. wiki `[[devsecvault-altcha-spi]]`).

Sans signal de pic, **aucune friction utilisateur**. L'utilisateur ne voit rien.

## Configuration Keycloak SPI Altcha (réutilisé depuis DSV)

Le SPI custom Java déjà packagé pour DevSecVault :

1. Copier le `.jar` Altcha SPI depuis le repo DSV (`spi/altcha/build/libs/altcha-spi-*.jar`) vers `infra/keycloak/providers/` du VPS.
2. Restart Keycloak (la stack baoifiée).
3. Realm `kalidoku` → **Authentication** → **Flows** → **Registration** → ajouter sous-flow "Altcha".
4. Authentication → Required Actions → activer "Altcha PoW".
5. Configurer `ALTCHA_HMAC_KEY` partagée entre le SPI et le backend kalidoku via Bao path `secret/_shared/altcha` (ou path par realm).

**TODO** : copier la procédure exacte de DSV dans `infra/keycloak/altcha-spi-deploy.md` quand on attaquera la phase auth.

## Liens

- Spec Altcha v2 : https://altcha.org/docs/v2/proof-of-work/
- SPI Keycloak DSV (réutilisable) : wiki `[[devsecvault-altcha-spi]]`
