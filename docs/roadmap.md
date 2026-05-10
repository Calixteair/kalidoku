# kalidoku — roadmap chiffrée

Estimations en **jours-homme senior** par phase. À multiplier × 1,5 si l'agent débute sur la stack.

## Phase 0 — Bootstrapping (✅ FAIT par ce scaffold)

- Repo, CLAUDE.md, README, LICENSE, .gitignore.
- Contrats fondateurs (entity-schema + openapi).
- Cargo workspace + squelettes Rust.
- Frontend Astro+Svelte squelette.
- Domain pack `paris-metro` (10 entités seed).
- Dockerfiles + compose.
- Configs Bao (policy, agent.hcl, setup.md).
- GitHub Actions complet.
- Issue/PR templates.
- Roadmap par agent (`docs/agents/agent-{a..f}.md`).

## Phase 1 — Premier domaine jouable (3 semaines, parallélisable)

| Agent | Tâches | Effort |
|---|---|---|
| **A** core | Familles de prédicats + loader + générateur CSP + tests | 5 j |
| **B** worker | Cron daily + once + idempotence + tests | 3 j |
| **C** server | Schéma DB + auth Keycloak + game endpoints + leaderboard MVP | 8 j |
| **D** web | Home + Grid + Autocomplete + EndGame + i18n FR/EN + PWA | 7 j |
| **E** infra | DH push opérationnel, NPM proxy host, Bao up, deploy SSH validé | 3 j |
| **F** domains | Ingest GTFS+OSM+Wikidata → 300 stations, validation schema | 4 j |

**Critère sortie phase 1** : `https://kalidoku.calixteair.fr` joue la grille du jour `paris-metro` sur mobile + desktop, leaderboard public, login Google + Discord.

## Phase 2 — Modes étendus (2 semaines)

- **Solo aléatoire** (Agent C + Agent D) : queue Redis, génération à la demande, gating quota free-tier, UI bouton "nouvelle grille". 4 j.
- **Score d'originalité (rareté live)** (Agent C) : agrégat des plays journaliers, push valeur en réponse de `/play`. 3 j.
- **Duel asynchrone** (Agent C + Agent D) : `POST /duels`, share URL HMAC-signé, page `/duel/[id]`. 4 j.
- **Stripe Checkout + premium** (Agent C, optionnel) : webhook, `users.premium_active`, gating quota. 3 j.

## Phase 3 — Multi-domaines (1 semaine par domaine)

Réplication du pattern `paris-metro` (Agent F + ajustements Agent A pour nouveaux prédicats si besoin) :

- `rer` (Agent F : 2 j).
- `transilien` (Agent F : 3 j).
- `sncf-grandes-lignes` (Agent F : 3 j).
- `world-airports` (Agent F : 4 j) — premier vrai gros dataset, tester MiniSearch côté autocomplete.

## Phase 4 — App mobile native (1 semaine)

- Capacitor wrap (Agent D + Agent E) : 2 j.
- Build iOS via GitHub Actions macos-runner : 2 j (challenge : signing).
- Submission Play Store + App Store : 3 j (review).

## Phase 5 — Hors transports (validation modularité)

Un domaine "fun" pour prouver que le moteur est bien générique :

- `pokemon-gen1` ou `nba-active-players` ou `marvel-cinematic-universe`.
- Effort : 3-5 j (dataset Wikidata + nouveaux prédicats si besoin).

## Risques

| Risque | Impact | Mitigation |
|---|---|---|
| Ingestion GTFS fragile | -3 j sur Phase 1 | Cache local des sources, fallback dataset minimal en attendant |
| Keycloak realm `kalidoku` config | -2 j Phase 1 | Procédure documentée dès le début, créer realm en Phase 0+ |
| Bundle web > 100 ko | UX dégradée mobile | Codesplitting Astro îlots, audit régulier `astro build --analyze` |
| Triche détectée publique | Réputation | Pattern backend autoritatif déjà en place, pas de mitigation supplémentaire MVP |
| OpenBao sealed après reboot | Site KO | Wazuh alerte déjà en place (Phase 1 SOC) |
