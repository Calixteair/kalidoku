# NPM (Nginx Proxy Manager) — proxy host kalidoku.calixteair.fr

État : **à faire manuellement** par toi. NPM exige un email/password admin que je n'ai pas accès en automation.

## Procédure

1. Aller sur https://npm.calixteair.fr (avec l'admin `kadmin` configuré).
2. **Hosts** → **Proxy Hosts** → **Add Proxy Host**.

### Onglet "Details"

| Field | Value |
|---|---|
| Domain Names | `kalidoku.calixteair.fr` |
| Scheme | `http` |
| Forward Hostname / IP | `kalidoku-web` |
| Forward Port | `80` |
| Cache Assets | ✅ |
| Block Common Exploits | ✅ |
| Websockets Support | ✅ |

### Onglet "Custom locations"

Ajouter une location pour passer `/api/*` au backend Rust :

| Field | Value |
|---|---|
| Define location | `/api/` |
| Scheme | `http` |
| Forward Hostname / IP | `kalidoku-server` |
| Forward Port | `8080` |
| Advanced | (voir bloc ci-dessous) |

```nginx
proxy_set_header Host $host;
proxy_set_header X-Real-IP $remote_addr;
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
proxy_set_header X-Forwarded-Proto $scheme;
proxy_read_timeout 60s;
```

### Onglet "SSL"

| Field | Value |
|---|---|
| SSL Certificate | **Request a new SSL Certificate** |
| Force SSL | ✅ |
| HTTP/2 Support | ✅ |
| HSTS Enabled | ✅ |
| HSTS Subdomains | ✅ |
| Use a DNS Challenge | ❌ (HTTP-01 OK pour ce sous-domaine) |
| Email | `reymond.calixte@gmail.com` |
| I Agree to Let's Encrypt TOS | ✅ |

### Onglet "Advanced"

```nginx
# Security headers (kalidoku, mobile-first PWA)
add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
add_header X-Frame-Options "DENY" always;
add_header X-Content-Type-Options "nosniff" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
add_header Permissions-Policy "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()" always;

# CSP : tighten when the SPA is in place. Keeping a permissive baseline now.
# Altcha PoW is self-hosted: no third-party origin needed in script-src/connect-src.
add_header Content-Security-Policy "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self'" always;
```

3. Sauvegarder. Le cert Let's Encrypt s'émet en ~10s.
4. Vérifier : `curl -sSI https://kalidoku.calixteair.fr` → 200 (ou 502 si les containers ne sont pas encore up — normal).

## Vérification finale

```bash
# Once kalidoku stack is deployed, this should return 200
curl -sS https://kalidoku.calixteair.fr/api/health | jq .
```
