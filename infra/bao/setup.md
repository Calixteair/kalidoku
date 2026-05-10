# OpenBao setup pour kalidoku

Procédure one-shot pour préparer OpenBao avant le premier déploiement. Réutilise les conventions et le pattern `bao-agent` Option B documentés dans wiki `[[vps-chantier-bao-migration]]` et `[[vps-wazuh-deployment-phase1]]`.

## 0. Prérequis

- OpenBao up + unsealed sur `https://secrets.calixteair.fr`.
- Token root accessible via `~/.config/openbao/init-secrets.json` sur le PC dev.
- Stack `bao-agent@.service` template déjà installée sur le VPS (héritage des stacks précédentes).
- Script `/usr/local/sbin/bao-deploy.sh` présent (héritage).

## 1. Push de la policy

```bash
ROOT_TOKEN=$(jq -r .root_token ~/.config/openbao/init-secrets.json)

# Depuis le repo cloné localement
ssh vps-claude "sudo docker exec -i -e BAO_TOKEN=$ROOT_TOKEN openbao bao policy write kalidoku-prod-read -" \
  < infra/bao/policy.hcl
```

## 2. Création de l'AppRole

```bash
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao write auth/approle/role/kalidoku-prod \
  token_policies=kalidoku-prod-read \
  token_ttl=20m \
  token_max_ttl=2h \
  secret_id_ttl=0 \
  secret_id_num_uses=0"
```

## 3. Récupération du role_id (jamais sensible, peut vivre en dur)

```bash
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao read \
  -field=role_id auth/approle/role/kalidoku-prod/role-id" \
  | sudo ssh vps-claude "tee /etc/bao/kalidoku/role_id >/dev/null && chmod 0640 /etc/bao/kalidoku/role_id"
```

## 4. Push des secrets (alphanumeric uniquement pour les passwords DB — gotcha YAML/sed/$)

```bash
# DB
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv put secret/kalidoku/prod/db \
  POSTGRES_DB=kalidoku \
  POSTGRES_USER=kalidoku \
  POSTGRES_PASSWORD=$(openssl rand -base64 24 | tr -dc 'A-Za-z0-9' | head -c 32)"

# Cache (Valkey)
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv put secret/kalidoku/prod/cache \
  VALKEY_PASSWORD=$(openssl rand -base64 24 | tr -dc 'A-Za-z0-9' | head -c 32)"

# Backend (HMAC keys)
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv put secret/kalidoku/prod/backend \
  SESSION_HMAC_KEY=$(openssl rand -base64 48) \
  PLAY_TOKEN_HMAC_KEY=$(openssl rand -base64 32) \
  RUST_LOG=info,kalidoku=debug,tower_http=info"

# OIDC Keycloak (à compléter une fois le client créé dans le realm kalidoku)
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv put secret/kalidoku/prod/oidc \
  KEYCLOAK_ISSUER_URL=https://auth.calixteair.fr/realms/kalidoku \
  KEYCLOAK_CLIENT_ID=kalidoku-web \
  KEYCLOAK_CLIENT_SECRET=__paste-from-keycloak-admin-ui__ \
  KEYCLOAK_REDIRECT_URL=https://kalidoku.calixteair.fr/api/auth/callback"

# Anti-bot — Altcha PoW HMAC key (self-hosted, no third party)
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv put secret/kalidoku/prod/anti-bot \
  ALTCHA_HMAC_KEY=$(openssl rand -base64 48 | tr -d '\n')"

# Search — Meilisearch master key (alphanumeric only, no $/:/&/*/#).
# Cette clé sert au server (proxy autocomplete) et au worker (indexation).
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv put secret/kalidoku/prod/search \
  MEILI_MASTER_KEY=$(openssl rand -base64 48 | tr -dc 'A-Za-z0-9' | head -c 64)"
```

> Rotation ultérieure : `bao kv patch secret/kalidoku/prod/search MEILI_MASTER_KEY=...`.
> Voir `docs/runbooks/meilisearch-ops.md` §"Rotation de la master key".

## 5. Déploiement de l'agent.hcl

```bash
scp infra/bao/agent.hcl vps-claude:/tmp/agent.hcl
ssh vps-claude "sudo install -o root -g root -m 0640 /tmp/agent.hcl /etc/bao/kalidoku/agent.hcl && rm /tmp/agent.hcl"
```

## 6. Premier wrap secret_id (TTL 10 min)

```bash
ssh vps-claude "sudo bash -c '
  WRAP=\$(docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao write -wrap-ttl=10m -force -field=wrapping_token \
    auth/approle/role/kalidoku-prod/secret-id)
  echo \"\$WRAP\" > /etc/bao/kalidoku/secret_id_wrap
  chmod 0600 /etc/bao/kalidoku/secret_id_wrap
'"
```

## 7. Activation du service

```bash
ssh vps-claude "sudo systemctl enable --now bao-agent@kalidoku.service"
ssh vps-claude "sudo systemctl status bao-agent@kalidoku.service --no-pager"
ssh vps-claude "sudo ls -la /run/kalidoku/"
```

Tu dois voir `/run/kalidoku/.env` (caché à `ls` simple, faire `ls -la`).

## 8. JWT auth GitHub Actions OIDC (CI Docker push)

La méthode JWT est déjà configurée dans `auth/jwt`, on ajoute juste un role pour ce repo :

```bash
cat > /tmp/gha-role.json <<EOF
{
  "role_type": "jwt",
  "user_claim": "actor",
  "bound_claims_type": "glob",
  "bound_claims": { "repository": "Calixteair/kalidoku" },
  "bound_audiences": "https://github.com/Calixteair",
  "policies": "gha-kalidoku-read",
  "ttl": "10m"
}
EOF

cat > /tmp/gha-policy.hcl <<EOF
path "secret/data/cicd/dockerhub" { capabilities = ["read"] }
EOF

scp /tmp/gha-role.json /tmp/gha-policy.hcl vps-claude:/tmp/

ssh vps-claude "sudo docker exec -i -e BAO_TOKEN=$ROOT_TOKEN openbao bao policy write gha-kalidoku-read - < /tmp/gha-policy.hcl"
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao write auth/jwt/role/gha-kalidoku @/tmp/gha-role.json"
```

## 9. Validation end-to-end

```bash
# Le service doit avoir rendu le .env
ssh vps-claude "sudo cat /run/kalidoku/.env | head -5"

# Test redéploiement standard
ssh vps-claude "sudo /usr/local/sbin/bao-deploy.sh kalidoku"

# Containers up + healthy
ssh vps-claude "sudo docker ps --filter name=kalidoku- --format 'table {{.Names}}\t{{.Status}}'"
```
