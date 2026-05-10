# Agent E — `infra/` + CI

## Périmètre

`infra/`, `.github/workflows/`, `.github/renovate.json`, `release-please-config.json`.

## Livrables

### 1. Dockerfiles (déjà posés, à compléter)

- `Dockerfile.server` : multi-stage cargo-chef, runtime distroless nonroot, healthcheck.
- `Dockerfile.worker` : idem.
- `Dockerfile.web` : pnpm build → nginx alpine.
- Vérifier que `cargo-chef` cache hits correctement (build < 2 min sur main avec cache GHA).

### 2. Compose

- `infra/docker/compose.yaml` : prod, postgres + valkey + server + worker + web sur réseaux `kalidoku-internal` (privé) + `nginx-reverse-proxy` (existant).
- `infra/docker/compose.dev.yaml` : DB + cache uniquement, le reste tourne sur l'hôte.

### 3. Bao

- `infra/bao/policy.hcl` : policy `kalidoku-prod-read`.
- `infra/bao/agent.hcl` : template bao-agent Option B.
- `infra/bao/setup.md` : runbook complet (déjà posé).

### 4. CI

- `ci.yml` : rust lint + test, web lint + test, gitleaks, trivy fs.
- `build-push.yml` : OIDC vers OpenBao pour Docker Hub creds, build matrix server/worker/web, push, trivy image.
- `deploy.yml` : SSH `ci-kalidoku` → ForceCommand → `bao-deploy.sh kalidoku` + smoke test `/api/health`.
- `release-please.yml` : auto PR de release.

### 5. Renovate

- Auto-merge minor/patch en green CI, major manuel.
- Schedule lundi matin pour limiter le bruit.

### 6. Scripts ops `infra/scripts/`

À créer :

- `pg-snapshot.sh` : `pg_dump | gpg --symmetric` chaque nuit, retention 30 jours.
- `bao-rotate-hmac.sh` : rotate `SESSION_HMAC_KEY` + `PLAY_TOKEN_HMAC_KEY`.

### 7. NPM

- Configurer le proxy host `kalidoku.calixteair.fr` dans NPM admin :
  - Forward `web` container port 80.
  - Forward `/api/*` vers `server` container port 8080 (location subroute).
  - Cert Let's Encrypt auto.
  - HSTS preload activé.

## Acceptance

- [ ] `docker compose -f infra/docker/compose.dev.yaml up -d` lance Postgres + Valkey clean.
- [ ] CI verte sur PR vide (squelettes seuls).
- [ ] `bao-deploy.sh kalidoku` sur le VPS lance les 4 containers, healthchecks all green.
- [ ] `https://kalidoku.calixteair.fr/api/health` retourne `{"status":"ok"}`.
