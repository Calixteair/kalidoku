# bao-agent template, deployed to /etc/bao/kalidoku/agent.hcl on the VPS.
# Renders /run/kalidoku/.env consumed by docker compose.
#
# Pattern: Option B (long-running systemd service per stack).
# See wiki [[vps-wazuh-deployment-phase1]] §"Conventions standardisées".

pid_file = "/run/kalidoku/agent.pid"

vault {
  address = "https://secrets.calixteair.fr"
}

auto_auth {
  method "approle" {
    config = {
      role_id_file_path                   = "/etc/bao/kalidoku/role_id"
      secret_id_file_path                 = "/etc/bao/kalidoku/secret_id_wrap"
      secret_id_response_wrapping_path    = "auth/approle/role/kalidoku-prod/secret-id"
      remove_secret_id_file_after_reading = true
    }
  }

  sink "file" {
    config = {
      path = "/run/kalidoku/.bao-token"
      mode = 0600
    }
  }
}

template {
  destination = "/run/kalidoku/.env"
  perms       = "0640"
  contents    = <<EOT
{{ with secret "secret/data/kalidoku/prod/db" }}
POSTGRES_DB={{ .Data.data.POSTGRES_DB }}
POSTGRES_USER={{ .Data.data.POSTGRES_USER }}
POSTGRES_PASSWORD={{ .Data.data.POSTGRES_PASSWORD }}
{{ end }}
{{ with secret "secret/data/kalidoku/prod/cache" }}
VALKEY_PASSWORD={{ .Data.data.VALKEY_PASSWORD }}
{{ end }}
{{ with secret "secret/data/kalidoku/prod/backend" }}
SESSION_HMAC_KEY={{ .Data.data.SESSION_HMAC_KEY }}
PLAY_TOKEN_HMAC_KEY={{ .Data.data.PLAY_TOKEN_HMAC_KEY }}
RUST_LOG={{ .Data.data.RUST_LOG }}
{{ end }}
{{ with secret "secret/data/kalidoku/prod/oidc" }}
KEYCLOAK_ISSUER_URL={{ .Data.data.KEYCLOAK_ISSUER_URL }}
KEYCLOAK_CLIENT_ID={{ .Data.data.KEYCLOAK_CLIENT_ID }}
KEYCLOAK_CLIENT_SECRET={{ .Data.data.KEYCLOAK_CLIENT_SECRET }}
KEYCLOAK_REDIRECT_URL={{ .Data.data.KEYCLOAK_REDIRECT_URL }}
{{ end }}
{{ with secret "secret/data/kalidoku/prod/anti-bot" }}
HCAPTCHA_SECRET={{ .Data.data.HCAPTCHA_SECRET }}
{{ end }}
EOT
}
