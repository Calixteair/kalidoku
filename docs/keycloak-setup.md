# Keycloak — setup kalidoku

État actuel (réalisé en autonomie):

- ✅ Realm `kalidoku` créé sur `https://auth.calixteair.fr`
- ✅ Client OIDC `kalidoku-web` créé (Authorization Code + PKCE S256, confidential)
- ✅ `KEYCLOAK_CLIENT_SECRET` patché dans OpenBao (`secret/kalidoku/prod/oidc`)
- ⏳ Identity providers Google + Discord — **à terminer manuellement** (étapes ci-dessous)
- ⏳ Theme custom hCaptcha — phase 2

## Ce qu'il reste à faire manuellement

### 1. Identity Provider Google

#### a. Créer l'OAuth client côté Google

1. Aller sur https://console.cloud.google.com/apis/credentials.
2. Créer (ou sélectionner) un projet "kalidoku".
3. **OAuth consent screen** → User Type: **External**.
   - App name: `kalidoku`
   - User support email: `reymond.calixte@gmail.com`
   - Developer contact: `reymond.calixte@gmail.com`
   - Authorized domains: `calixteair.fr`
4. **Credentials** → **Create credentials** → **OAuth client ID**.
   - Application type: **Web application**
   - Name: `kalidoku Keycloak`
   - Authorized JavaScript origins: `https://auth.calixteair.fr`
   - Authorized redirect URIs:
     - `https://auth.calixteair.fr/realms/kalidoku/broker/google/endpoint`
5. Récupérer **Client ID** et **Client secret**.

#### b. Configurer dans Keycloak

```bash
KC_TOKEN=$(curl -sS -X POST "https://auth.calixteair.fr/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin" --data-urlencode "password=$KC_ADMIN_PASS" \
  -d "grant_type=password" -d "client_id=admin-cli" | jq -r .access_token)

GOOGLE_CLIENT_ID="<paste>"
GOOGLE_CLIENT_SECRET="<paste>"

curl -sS -X POST "https://auth.calixteair.fr/admin/realms/kalidoku/identity-provider/instances" \
  -H "Authorization: Bearer $KC_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "google",
    "providerId": "google",
    "enabled": true,
    "trustEmail": true,
    "storeToken": false,
    "addReadTokenRoleOnCreate": false,
    "firstBrokerLoginFlowAlias": "first broker login",
    "config": {
      "clientId": "'"$GOOGLE_CLIENT_ID"'",
      "clientSecret": "'"$GOOGLE_CLIENT_SECRET"'",
      "syncMode": "IMPORT",
      "useJwksUrl": "true"
    }
  }'
```

### 2. Identity Provider Discord

Discord n'est pas un built-in provider Keycloak, mais Keycloak supporte **OpenID Connect generic** ou **OAuth 2.0 generic**. Discord expose un OAuth2 standard.

#### a. Créer l'app côté Discord

1. Aller sur https://discord.com/developers/applications.
2. **New Application** → name `kalidoku`.
3. **OAuth2** → ajouter redirect URI: `https://auth.calixteair.fr/realms/kalidoku/broker/discord/endpoint`.
4. Récupérer **Client ID** et **Client Secret** (onglet OAuth2).

#### b. Configurer dans Keycloak (provider OAuth2 generic)

```bash
DISCORD_CLIENT_ID="<paste>"
DISCORD_CLIENT_SECRET="<paste>"

curl -sS -X POST "https://auth.calixteair.fr/admin/realms/kalidoku/identity-provider/instances" \
  -H "Authorization: Bearer $KC_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "discord",
    "displayName": "Discord",
    "providerId": "oidc",
    "enabled": true,
    "trustEmail": true,
    "storeToken": false,
    "firstBrokerLoginFlowAlias": "first broker login",
    "config": {
      "clientId": "'"$DISCORD_CLIENT_ID"'",
      "clientSecret": "'"$DISCORD_CLIENT_SECRET"'",
      "tokenUrl": "https://discord.com/api/oauth2/token",
      "authorizationUrl": "https://discord.com/api/oauth2/authorize",
      "userInfoUrl": "https://discord.com/api/users/@me",
      "defaultScope": "identify email",
      "syncMode": "IMPORT",
      "validateSignature": "false",
      "clientAuthMethod": "client_secret_post"
    }
  }'
```

Discord n'expose pas `email_verified` standard OIDC : depuis Keycloak, configurer un mapper pour `email` issu de l'API user info.

### 3. Smoke test

Après ces deux étapes :

```bash
# Login flow doit lister "Sign in with Google" + "Sign in with Discord"
xdg-open "https://auth.calixteair.fr/realms/kalidoku/account"
```

## Notes sécurité

- L'admin password Keycloak est dans `secret/keycloak/prod/app` (Bao). Source de vérité.
- Les secrets Google + Discord doivent **aussi** être pushés dans Bao après création (`secret/kalidoku/prod/idp` → `kv put GOOGLE_CLIENT_SECRET=… DISCORD_CLIENT_SECRET=…`) pour traçabilité, même si Keycloak les stocke déjà en interne.
- Realm `verifyEmail: true` est activé — sauf pour les comptes Google (déjà vérifiés via `trustEmail: true`).
- `bruteForceProtected: true`, lockout après 8 échecs en 12h.
