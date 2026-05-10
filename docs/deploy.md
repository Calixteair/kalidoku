# Déploiement kalidoku

## Pipeline (résumé)

```
push main
    │
    ├── ci.yml ──── lint + tests + trivy fs (bloquant)
    │
    └── build-push.yml ─── OIDC OpenBao → Docker Hub creds
                            ├── build kalidoku-server
                            ├── build kalidoku-worker
                            ├── build kalidoku-web
                            └── trivy image (bloquant CRITICAL)
                                    │
                                    ▼
                           deploy.yml (workflow_call)
                                    │
                                    ▼
                  SSH ci-kalidoku@calixteair.fr (ForceCommand)
                                    │
                                    ▼
                  /usr/local/sbin/ci-deploy-self.sh
                                    │
                                    ▼
                  /usr/local/sbin/bao-deploy.sh kalidoku
                                    │
                                    ▼
                  bao-agent renders /run/kalidoku/.env
                                    │
                                    ▼
                  docker compose --env-file ... up -d
                                    │
                                    ▼
                  smoke test https://kalidoku.calixteair.fr/api/health
```

## Setup initial (one-shot, à faire avant le premier deploy)

### 1. Realm Keycloak

```bash
# Sur l'admin Keycloak : créer realm kalidoku
# Ajouter client OIDC "kalidoku-web" : Authorization Code + PKCE, redirect URIs :
#   https://kalidoku.calixteair.fr/api/auth/callback
#   http://localhost:8080/api/auth/callback (dev)
# Activer Identity Providers : Google, Discord (configurer creds OAuth)
# Récupérer client_secret → push dans Bao secret/kalidoku/prod/oidc
```

### 2. OpenBao (cf. `infra/bao/setup.md`)

Push policy + AppRole + secrets + role JWT GHA.

### 3. User SSH ci-kalidoku (cf. wiki `[[vps-ci-deployers-pattern]]`)

```bash
sudo getent group ci-deployers >/dev/null || sudo groupadd ci-deployers
sudo useradd --create-home --shell /usr/sbin/nologin --gid ci-deployers --groups docker ci-kalidoku
ssh-keygen -t ed25519 -C "ci-kalidoku@github-actions" -f ~/.config/vps-ci-keys/ci-kalidoku -N ""
# Push pubkey + ForceCommand → /home/ci-kalidoku/.ssh/authorized_keys
# Add /etc/sudoers.d/ci-deployers (cf. wiki)
# Add /usr/local/sbin/ci-deploy-self.sh (cf. wiki)
```

### 4. GitHub Secrets (Settings → Secrets → Actions)

| Secret | Valeur |
|---|---|
| `DEPLOY_HOST` | `calixteair.fr` |
| `DEPLOY_PORT` | port SSH |
| `DEPLOY_USER` | `ci-kalidoku` |
| `DEPLOY_SSH_KEY` | contenu de `~/.config/vps-ci-keys/ci-kalidoku` |

(Les creds Docker Hub sont récupérées via OIDC OpenBao, pas en GitHub Secrets.)

### 5. NPM proxy host

Dans NPM admin :
- Domain `kalidoku.calixteair.fr` → forward to `kalidoku-web:80`.
- Custom location `/api/` → forward to `kalidoku-server:8080` (preserve path).
- Let's Encrypt auto cert.
- Force SSL + HSTS preload.

### 6. Premier déploiement manuel

```bash
ssh vps-claude
sudo mkdir -p /home/calixteair/docker/projects/kalidoku
# Push compose.yaml dans ce dossier (par scp ou clone du repo)
sudo /usr/local/sbin/bao-deploy.sh kalidoku
```

## Rollback

Pour revenir à un commit précédent :

```bash
ssh vps-claude
sudo bash -c '
  cd /home/calixteair/docker/projects/kalidoku
  KALIDOKU_SERVER_IMAGE=calixteair/kalidoku-server:sha-abc1234 \
  KALIDOKU_WORKER_IMAGE=calixteair/kalidoku-worker:sha-abc1234 \
  KALIDOKU_WEB_IMAGE=calixteair/kalidoku-web:sha-abc1234 \
  /usr/local/sbin/bao-deploy.sh kalidoku
'
```

Watchtower n'est volontairement pas utilisé : on contrôle les pulls via tag immutable.

## Procédures incident

Voir `docs/security.md` §"Procédures incident".
