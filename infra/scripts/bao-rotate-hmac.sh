#!/usr/bin/env bash
# bao-rotate-hmac.sh — rotate kalidoku HMAC keys via OpenBao kv patch.
#
# Rotates the three runtime HMAC secrets used by the backend:
#   - SESSION_HMAC_KEY    (signs the __Host-session cookie token)
#   - PLAY_TOKEN_HMAC_KEY (signs the per-game play_token)
#   - ALTCHA_HMAC_KEY     (signs Altcha PoW challenges)
#
# Steps:
#   1. Generate three fresh random keys (48 / 32 / 48 bytes base64).
#   2. `kv patch` them into their existing KV paths (NEVER kv put — would wipe
#      sibling keys like RUST_LOG or KEYCLOAK_*).
#   3. Re-wrap the AppRole secret_id (TTL 10m) so bao-agent can re-auth after
#      restart.
#   4. systemctl restart bao-agent@kalidoku.service so the rendered .env picks
#      up the new values.
#   5. Trigger `bao-deploy.sh kalidoku` so the server container restarts and
#      re-reads SESSION_HMAC_KEY at boot.
#
# Caveat: rotating SESSION_HMAC_KEY immediately invalidates every active
# session (users get logged out). Schedule during low-traffic windows.
#
# Required env:
#   ROOT_TOKEN  — OpenBao root token (or any token with sudo on
#                 secret/data/kalidoku/prod/* and
#                 auth/approle/role/kalidoku-prod/secret-id).
#                 Read once from ~/.config/openbao/init-secrets.json on the
#                 admin workstation, never persisted on the VPS.
#
# Run as root on the VPS (needs systemctl + bao-deploy.sh).
#
# Usage examples:
#   sudo ROOT_TOKEN=$(jq -r .root_token ~/.config/openbao/init-secrets.json) \
#        ./bao-rotate-hmac.sh
#   sudo ROOT_TOKEN=... ./bao-rotate-hmac.sh --dry-run

set -euo pipefail

STACK="kalidoku"
APPROLE="kalidoku-prod"
KV_PATH="secret/kalidoku/prod/backend"
ANTIBOT_PATH="secret/kalidoku/prod/anti-bot"
BAO_CONTAINER="${BAO_CONTAINER:-openbao}"
BAO_DIR="/etc/bao/${STACK}"
SERVICE="bao-agent@${STACK}.service"
DEPLOY_SCRIPT="/usr/local/sbin/bao-deploy.sh"

DRY_RUN=0
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=1

log() { printf '[bao-rotate-hmac] %s\n' "$*"; }
fatal() { log "FATAL: $*"; exit 1; }

#-----------------------------
# Pre-flight
#-----------------------------
[[ $EUID -eq 0 ]]                 || fatal "must run as root"
[[ -n "${ROOT_TOKEN:-}" ]]        || fatal "ROOT_TOKEN env var required"
command -v docker >/dev/null      || fatal "docker missing"
command -v systemctl >/dev/null   || fatal "systemctl missing"
[[ -d "$BAO_DIR" ]]               || fatal "$BAO_DIR not found — bao-agent not initialized?"
[[ -x "$DEPLOY_SCRIPT" ]]         || fatal "$DEPLOY_SCRIPT not executable"

bao_exec() {
    docker exec -e BAO_TOKEN="$ROOT_TOKEN" "$BAO_CONTAINER" bao "$@"
}

#-----------------------------
# 1. Generate fresh keys
#-----------------------------
SESSION_KEY="$(openssl rand -base64 48 | tr -d '\n')"
PLAY_KEY="$(openssl rand -base64 32 | tr -d '\n')"
ALTCHA_KEY="$(openssl rand -base64 48 | tr -d '\n')"

log "generated 3 fresh HMAC keys (lengths: $(echo -n "$SESSION_KEY" | wc -c)/$(echo -n "$PLAY_KEY" | wc -c)/$(echo -n "$ALTCHA_KEY" | wc -c))"

if (( DRY_RUN )); then
    log "DRY-RUN — would patch $KV_PATH (SESSION_HMAC_KEY, PLAY_TOKEN_HMAC_KEY) and $ANTIBOT_PATH (ALTCHA_HMAC_KEY)"
    log "DRY-RUN — would re-wrap secret_id and restart $SERVICE"
    log "DRY-RUN — would invoke $DEPLOY_SCRIPT $STACK"
    exit 0
fi

#-----------------------------
# 2. kv patch (additive — preserves RUST_LOG and friends)
#-----------------------------
log "patching $KV_PATH"
bao_exec kv patch "$KV_PATH" \
    "SESSION_HMAC_KEY=$SESSION_KEY" \
    "PLAY_TOKEN_HMAC_KEY=$PLAY_KEY"

log "patching $ANTIBOT_PATH"
bao_exec kv patch "$ANTIBOT_PATH" \
    "ALTCHA_HMAC_KEY=$ALTCHA_KEY"

#-----------------------------
# 3. Re-wrap secret_id
#-----------------------------
# `remove_secret_id_file_after_reading = true` in agent.hcl means the previous
# wrap was already consumed. Without a fresh wrap, systemctl restart would
# crash-loop on `missing or empty .../secret_id_wrap`.
log "re-wrapping AppRole secret_id (TTL 10m)"
WRAP="$(bao_exec write -wrap-ttl=10m -force \
            -field=wrapping_token \
            "auth/approle/role/${APPROLE}/secret-id")"
[[ -n "$WRAP" ]] || fatal "empty wrap returned by openbao"

install -m 0600 -o root -g root /dev/null "$BAO_DIR/secret_id_wrap"
printf '%s' "$WRAP" > "$BAO_DIR/secret_id_wrap"

#-----------------------------
# 4. Restart bao-agent → re-render /run/kalidoku/.env
#-----------------------------
log "restarting $SERVICE"
systemctl restart "$SERVICE"
# Give the agent a few seconds to render the new .env file.
for _ in 1 2 3 4 5; do
    sleep 2
    if [[ -s "/run/${STACK}/.env" ]] && grep -q '^SESSION_HMAC_KEY=' "/run/${STACK}/.env"; then
        log "agent rendered new /run/${STACK}/.env"
        break
    fi
done

#-----------------------------
# 5. Re-deploy the stack so the server picks up the new env
#-----------------------------
log "redeploying ${STACK} via $DEPLOY_SCRIPT"
"$DEPLOY_SCRIPT" "$STACK"

log "rotation complete — sessions and play_tokens issued before this point are now invalid"
