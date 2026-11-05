# Déploiement kalidoku

## Pipeline (résumé)

```
PR                    Jenkins (jenkins.calixteair.fr, Jenkinsfile + bibliothèque jenkins-lib)
                        Rust : fmt, clippy, tests (Postgres + Valkey de test), cargo-audit, cargo-deny
                        web : pnpm lint, typecheck, tests ; gitleaks ; Snyk ; 3 images + Trivy
main                  idem, puis images <composant>-sha-<commit> et <composant>-latest sur Harbor
                      (registry.calixteair.fr/kalidoku/kalidoku), sans déploiement
release-please        PR de version (CHANGELOG, manifest), sans tag ni GitHub Release
tag vX.Y.Z signé      (Calixteair : git tag -s, après le merge de la PR de version)
    │                 images vX.Y.Z-server, vX.Y.Z-worker, vX.Y.Z-web (immuables)
    ▼
SSH ci-kalidoku@vps vX.Y.Z → ci-deploy-self.sh → stack-sync.sh kalidoku vX.Y.Z
    │   le VPS récupère infra/ au tag, vérifie la signature (clé de Calixteair) et que le
    │   commit est sur main, épingle les images sur vX.Y.Z-*, contrôle la politique
    ▼
infra/docker/compose.yaml + infra/bao/agent.hcl installés → bao-deploy.sh → santé
    (retour à la configuration précédente si un conteneur n'est pas sain)
```

Détails du déploiement signé : `vps-soc-config/stack-sync/README.md`.

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

### 4. CI et déploiement signé (Jenkins, Harbor, stack-sync)

Plus aucun secret dans GitHub. Côté `vps-soc-config` : politique `stack-sync/policies/kalidoku.json`,
signataires, clé publique `stack-sync/ci-keys/kalidoku.pub`, projet Harbor (`harbor/ci-robots.sh`),
puis `ansible-playbook … --tags stack_sync`. Côté OpenBao : `cicd/harbor/kalidoku` (robot) et
`cicd/kalidoku/deploy` (clé SSH de `ci-kalidoku`, lue par Jenkins).

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

Redéployer la release précédente (tag déjà signé, images immuables dans Harbor) :

```bash
ssh vps-claude 'sudo /usr/local/sbin/stack-sync.sh kalidoku vX.Y.Z'
```

Détails : `docs/runbooks/rollback.md`.

## Procédures incident

Voir `docs/security.md` §"Procédures incident".
