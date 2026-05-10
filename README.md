# kalidoku

Puzzle quotidien type *Connections* : remplis une grille 3×3 avec des entités qui satisfont à la fois le critère de leur ligne et celui de leur colonne. Mode lancement : **stations du métro parisien**. Architecture **multi-domaine** : ajout de RER, Transilien, SNCF, aéroports internationaux, et au-delà (Pokémon, films, NBA, etc.) sans toucher au moteur.

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Stack

| Couche | Tech |
|---|---|
| Moteur (lib) | Rust pur, `serde`, `thiserror` |
| Backend HTTP | Rust, **Axum**, **SeaORM**, Tokio |
| Worker génération | Rust (binaire séparé partageant `core/`) |
| Base de données | PostgreSQL 16 |
| Cache / queue | Valkey (fork OSS Redis) |
| Frontend | **Astro 4** + **Svelte 5** + Tailwind 4 + shadcn-svelte |
| i18n | paraglide-js (FR/EN) |
| Auth | **Keycloak** (`auth.calixteair.fr`), realm `kalidoku`, OIDC PKCE, Google + Discord + email/password |
| Secrets | **OpenBao** (`secrets.calixteair.fr`), bao-agent systemd Option B |
| Mobile | PWA + **Capacitor**-ready (web réutilisé tel quel pour iOS/Android plus tard) |
| Logging | `tracing` JSON → Docker stdout → agent Wazuh host (`wazuh.calixteair.fr`) |
| Reverse proxy | Nginx Proxy Manager (`npm.calixteair.fr`) |
| CI/CD | GitHub Actions → Docker Hub → SSH `ci-kalidoku` → `bao-deploy.sh` |

## Modes de jeu

### MVP

- **Daily** : grille du jour, 1 partie par device, leaderboard public.
- **Solo aléatoire** : 3 grilles/jour gratuites, génération à la demande.

### Phase 2 (gating prêt dans le code, pas activé)

- **Solo illimité** + **Duels entre amis illimités** → premium ~3-5 €/mois (Stripe).
- **Score d'originalité** (rareté pondérée live).

## Modules / dossiers

```
.
├── core/                    Rust lib pure (Entity, Predicate, Solver, Validator)
├── server/                  Backend Axum (API REST OpenAPI)
├── worker/                  Cron + génération à la demande
├── web/                     Astro + Svelte (front responsive mobile-first)
├── domains/
│   └── paris-metro/         Domain pack #1 (entities.json + predicates.json)
├── contracts/
│   ├── entity-schema.json   JSON Schema des entités/prédicats
│   └── openapi.yaml         Contrat HTTP backend ↔ front
├── infra/
│   ├── docker/              Dockerfiles
│   ├── bao/                 Policy + agent.hcl + scripts setup OpenBao
│   └── scripts/             Scripts ops (snapshot DB, etc.)
├── docs/                    Architecture, sécurité, runbooks
└── .github/                 Workflows CI/CD + templates
```

## Setup dev (local)

### Prérequis

- **Rust** stable + `cargo`, `rustfmt`, `clippy`
- **Node** 22+, **pnpm** 9+
- **Docker** + Docker Compose plugin
- **`sqlx-cli`** (`cargo install sqlx-cli --no-default-features --features postgres`)
- **`sea-orm-cli`** (`cargo install sea-orm-cli`)
- **`gitleaks`** (`brew install gitleaks` ou équivalent)

### Premier démarrage

```bash
# 1. Cloner et installer
git clone git@github.com:Calixteair/kalidoku.git
cd kalidoku
pnpm --dir web install

# 2. Lancer Postgres + Valkey en local
docker compose -f infra/docker/compose.dev.yaml up -d

# 3. Setup DB
cd server
cp .env.example .env   # contient des creds dev locaux uniquement
sea-orm-cli migrate up

# 4. Lancer le backend en watch
cargo watch -x 'run -p kalidoku-server'

# 5. Dans un autre terminal, lancer le front
cd web && pnpm dev
```

L'API tourne sur `http://localhost:8080`, le front sur `http://localhost:4321`.

### Tests

```bash
# Rust
cargo test --workspace

# Front
cd web && pnpm test

# E2E (Playwright)
cd web && pnpm exec playwright test
```

### Lint avant commit

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cd web && pnpm lint && pnpm typecheck
gitleaks protect --staged
```

## Déploiement

Auto via GitHub Actions sur merge dans `main`. Voir `.github/workflows/` et `docs/deploy.md`.

## Contribuer

- Issues bienvenues, voir `.github/ISSUE_TEMPLATE/`.
- PRs : respecter `CLAUDE.md` (conventions, sécurité, mobile-first).
- Pas de mention d'IA dans commits/PR (cf. `CLAUDE.md` §2).

## Licence

[AGPL-3.0](LICENSE). Tout fork hébergé doit publier ses modifications.

## Crédits / inspiration

Concept inspiré de [métrodoku.fr](https://metrodoku.fr) (jeu solo non lié, code propriétaire). kalidoku est une réécriture from-scratch, multi-domaine, open source.
