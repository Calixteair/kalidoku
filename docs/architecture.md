# kalidoku — architecture

## Vue d'ensemble

```
                             ┌──────────────────────────────┐
            cron 00:01 UTC   │ kalidoku-worker (Rust)       │
            on-demand        │ - charge domain pack         │
                             │ - core::generator::generate  │
                             │ - INSERT grids in Postgres   │
                             └──────────────┬───────────────┘
                                            │
   ┌───────────────────────┐                ▼
   │ kalidoku-web (Astro)  │       ┌────────────────┐
   │  - PWA, mobile-first  │       │  Postgres 16   │
   │  - autocomplete (TS)  │ HTTPS │  (SeaORM)      │
   │  - Capacitor-ready    │◀────▶ ├────────────────┤
   └───────────┬───────────┘  REST │  Valkey 7      │
               │              JSON │  (rate-limit,  │
               │                   │   queue solo)  │
               ▼                   └────────┬───────┘
   ┌───────────────────────┐                ▲
   │ kalidoku-server (Rust)│  SeaORM        │
   │  - Axum 0.7           │ ──────────────┘
   │  - OIDC PKCE Keycloak │
   │  - HMAC play tokens   │
   │  - core::validator    │
   └───────────────────────┘
```

## Couches

### `core/` — moteur générique

- Pure Rust, **pas d'I/O, pas d'async**. C'est le cœur testable du jeu.
- Types : `Entity`, `AttributeValue`, `Predicate (trait)`, `Grid`, `GenerationOptions`.
- Modules : `entity`, `predicate`, `domain`, `generator`, `validator`, `normalize`, `search`.
- Le moteur **ne sait rien** des stations de métro. Il manipule des entités opaques + prédicats.

### `domains/<id>/` — domain packs

- `metadata.json` : identité du domaine.
- `predicates.json` : liste de prédicats parametrés (`{"family": "ends_with", "param": "S"}`).
- `entities.json` : liste d'entités avec leurs attributes.
- Optionnel : `scripts/ingest.{ts,py,rs}` pour rebuilder le dataset depuis les sources.
- Ajouter un domaine = **uniquement** ajouter un dossier ici. Aucun fichier `core/` à modifier.

### `worker/` — génération

- Cron 00:01 UTC pour les grilles `daily` (1 par domaine actif).
- File Redis pour les grilles `solo` à la demande.
- Idempotent : `INSERT ... ON CONFLICT DO NOTHING` sur `(domain, mode, publish_at)`.
- Multi-thread Tokio mais l'algo CSP lui-même est synchrone (depuis `core::generator::generate`).

### `server/` — API HTTP

- Axum, OpenAPI v1 dans `contracts/openapi.yaml`.
- Couches :
  - `routes/` : handlers fins, délèguent à `services/`.
  - `services/` : logique applicative (game engine wrapper, scoring, leaderboard agg).
  - `repos/` : accès DB via SeaORM.
  - `auth/` : OIDC PKCE Keycloak, sessions cookie HMAC.
  - `middleware/` : rate-limit `tower_governor`, CORS, headers sécu, tracing.
- **Anti-cheat** : les solutions ne quittent JAMAIS le serveur tant que la partie n'est pas finie.
  Le client envoie une réponse → serveur valide via `core::validator::validate_answer` → renvoie `{ok, scoreDelta, mistakesLeft}`.

### `web/` — frontend

- Astro 4 pour les pages statiques (home, FAQ, blog, leaderboards).
- Svelte 5 pour les îlots interactifs (la grille, l'autocomplete, les modals).
- Tailwind 4 + `shadcn-svelte` + `lucide-svelte`.
- i18n FR/EN via `paraglide-js`.
- PWA manifest + service worker, offline cache de la grille du jour.
- Capacitor-ready : `capacitor.config.ts` posé pour wrapper plus tard.

## Sécurité

Voir `docs/security.md`. Points-clés :

- **Backend autoritatif** : 0 solution exposée avant la fin de partie.
- **Cookies session** : `__Host-session`, HttpOnly, Secure, SameSite=Strict, hash SHA-256 stocké en DB.
- **Play token HMAC** : signe `{game_id, device_id, started_at}`, TTL 1h, requis pour chaque `/play`.
- **Rate limit** : `tower_governor` 1 req/250ms par device, 60 req/min par IP.
- **hCaptcha** : sur l'inscription Keycloak (Altcha SPI custom déjà éprouvée sur DSV — voir wiki) et sur démarrage de partie en cas de pic.
- **Secrets** : OpenBao, jamais en clair, charset password alphanumeric strict.

## Choix d'implémentation (ADR courts)

### ADR-001 — SeaORM plutôt que SQLx pur

Raison : tu m'as demandé "le plus opti pour la vélocité". SeaORM offre :
- Active record style sur les entities générées.
- Migrations Rust idiomatiques.
- Toujours basé sur sqlx en interne, donc perf équivalente sur les requêtes.

Coût : un peu plus de magie. Acceptable.

### ADR-002 — Valkey plutôt que Redis

Raison : Redis 7.4 a changé de licence (RSALv2/SSPL). Valkey est le fork OSS soutenu par la Linux Foundation. Drop-in remplaçant, **0 ligne de code à changer**.

### ADR-003 — Astro + Svelte plutôt que React/Angular

Raison : mobile-first, bundle minimal, pages statiques pour FAQ/blog, îlots Svelte pour la grille. Capacitor wrap natif avec **0 effort** côté code applicatif (le webview embarque le site Astro tel quel). Angular = trop lourd, React seul = pas de SSG natif sans Next.

### ADR-004 — Génération côté serveur uniquement

Raison : si le client génère, il connaît les solutions. Tout reste serveur, et le client reçoit une `PublicGrid` (sans `S_ij`).

### ADR-005 — pas de cosign / SBOM au MVP

Raison : Snyk fait le scan vuln en pré-release manuel, Trivy bloque les CRITICAL en CI. Le coût ops de cosign+SBOM dépasse le bénéfice tant qu'on n'a pas une exigence compliance.

## Liens wiki Obsidian (PC dev)

- `[[kalidoku-projet]]`
- `[[lab-openbao]]`
- `[[vps-chantier-bao-migration]]`
- `[[vps-ci-deployers-pattern]]`
- `[[vps-wazuh-deployment-phase1]]`
