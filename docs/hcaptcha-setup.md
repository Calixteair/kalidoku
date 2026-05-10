# hCaptcha — setup kalidoku

État actuel : `secret/kalidoku/prod/anti-bot` contient un placeholder `HCAPTCHA_SECRET`.

## Procédure

### 1. Créer un compte hCaptcha

1. Aller sur https://www.hcaptcha.com.
2. Sign up (free tier suffit pour démarrer).
3. **Add Site**.
   - Hostname: `kalidoku.calixteair.fr`
   - Difficulty: **Easy** au début (durcir si abus mesurés).
4. Récupérer la **Site Key** (publique, va dans le HTML/JS du front) et le **Secret Key** (côté serveur).

### 2. Push le Secret Key dans Bao

```bash
HCAPTCHA_SECRET="<paste-from-hcaptcha-dashboard>"
ROOT_TOKEN=$(jq -r .root_token ~/.config/openbao/init-secrets.json)
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv patch \
  secret/kalidoku/prod/anti-bot HCAPTCHA_SECRET=$HCAPTCHA_SECRET"
```

Re-wrap secret_id + restart `bao-agent@kalidoku.service` pour que la nouvelle valeur arrive dans `/run/kalidoku/.env` :

```bash
ssh vps-claude "sudo bash -c '
  WRAP=\$(docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao write -wrap-ttl=10m -force -field=wrapping_token auth/approle/role/kalidoku-prod/secret-id)
  printf %s \"\$WRAP\" > /etc/bao/kalidoku/secret_id_wrap
  chmod 600 /etc/bao/kalidoku/secret_id_wrap
  systemctl restart bao-agent@kalidoku.service
'"
```

### 3. Pousser la Site Key (publique) dans le repo

Pas un secret, mais besoin d'être versionné. Ajouter dans `web/.env.production` (committé) :

```
PUBLIC_HCAPTCHA_SITE_KEY=10000000-ffff-ffff-ffff-000000000001
```

Note : la valeur ci-dessus est la **clé publique de test hCaptcha** (toujours invisible, accepte tout). Remplacer par la vraie quand prête. Mettre à jour le composant Svelte qui fait le widget.

### 4. Validation côté backend

Dans `server/src/auth/captcha.rs` (à implémenter par agent C) :

```rust
async fn verify_hcaptcha(token: &str, ip: &str, secret: &str) -> bool {
    let resp: serde_json::Value = reqwest::Client::new()
        .post("https://api.hcaptcha.com/siteverify")
        .form(&[("secret", secret), ("response", token), ("remoteip", ip)])
        .send().await.ok().unwrap()
        .json().await.ok().unwrap();
    resp["success"].as_bool().unwrap_or(false)
}
```
