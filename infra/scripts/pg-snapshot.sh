#!/usr/bin/env bash
# pg-snapshot.sh — daily encrypted Postgres snapshot for kalidoku.
#
# Pipes `pg_dump --format=custom` through `gpg --symmetric` and stores the
# result under /home/calixteair/backups/kalidoku/. Old snapshots beyond
# RETENTION_DAYS days are pruned.
#
# Designed to run as the `calixteair` user via cron or a systemd timer:
#   0 3 * * *  /home/calixteair/docker/projects/kalidoku/infra/scripts/pg-snapshot.sh
#
# Required environment (sourced from /run/kalidoku/.env via the wrapper):
#   POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
#   PG_BACKUP_PASSPHRASE   — symmetric GPG passphrase, stored in OpenBao under
#                            secret/kalidoku/prod/backup, rendered alongside
#                            the rest of the env by bao-agent.
#
# Logs to stdout (captured by journald or cron) and to a tagged tag for Wazuh
# pickup via syslog (`logger -t kalidoku-backup`).

set -euo pipefail

#-----------------------------
# Defaults (overridable via env)
#-----------------------------
BACKUP_DIR="${BACKUP_DIR:-/home/calixteair/backups/kalidoku}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
COMPOSE_FILE="${COMPOSE_FILE:-/home/calixteair/docker/projects/kalidoku/compose.yaml}"
ENV_FILE="${ENV_FILE:-/run/kalidoku/.env}"
TAG="kalidoku-backup"

#-----------------------------
# Helpers
#-----------------------------
log() { logger -t "$TAG" -- "$*"; printf '[%s] %s\n' "$TAG" "$*"; }
fatal() { log "FATAL: $*"; exit 1; }

#-----------------------------
# Pre-flight
#-----------------------------
[[ -r "$ENV_FILE" ]] || fatal "env file not readable: $ENV_FILE"

# Source the rendered .env (POSIX-safe, single-line key=value pairs only).
# shellcheck disable=SC1090
set -a; . "$ENV_FILE"; set +a

: "${POSTGRES_USER:?POSTGRES_USER missing in $ENV_FILE}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD missing in $ENV_FILE}"
: "${POSTGRES_DB:?POSTGRES_DB missing in $ENV_FILE}"
: "${PG_BACKUP_PASSPHRASE:?PG_BACKUP_PASSPHRASE missing in $ENV_FILE — push it under secret/kalidoku/prod/backup}"

command -v gpg >/dev/null    || fatal "gpg not installed"
command -v docker >/dev/null || fatal "docker not installed"

mkdir -p "$BACKUP_DIR"
chmod 0700 "$BACKUP_DIR"

#-----------------------------
# Dump + encrypt (streamed)
#-----------------------------
TS=$(date -u +%Y%m%dT%H%M%SZ)
OUT="$BACKUP_DIR/kalidoku-${TS}.dump.gpg"
TMP="${OUT}.partial"

log "snapshot start → $OUT"

# pg_dump runs *inside* the postgres container — no need to expose 5432 to host.
# `--format=custom` is restorable with pg_restore and supports parallelism later.
# We disable the host-key prompt for non-interactive cron via --batch.
docker compose --file "$COMPOSE_FILE" --env-file "$ENV_FILE" \
    exec -T postgres \
    env "PGPASSWORD=$POSTGRES_PASSWORD" \
    pg_dump \
        --username "$POSTGRES_USER" \
        --dbname   "$POSTGRES_DB" \
        --format=custom \
        --no-owner \
        --no-privileges \
  | gpg --batch --yes --quiet --no-tty \
        --symmetric \
        --cipher-algo AES256 \
        --compress-algo zlib --compress-level 9 \
        --passphrase-fd 3 \
        --output "$TMP" \
    3<<<"$PG_BACKUP_PASSPHRASE"

mv "$TMP" "$OUT"
chmod 0600 "$OUT"

SIZE_KB=$(du -k "$OUT" | awk '{print $1}')
log "snapshot done size=${SIZE_KB}KiB file=$OUT"

#-----------------------------
# Retention prune
#-----------------------------
if [[ "$RETENTION_DAYS" -gt 0 ]]; then
    PRUNED=$(find "$BACKUP_DIR" -maxdepth 1 -type f -name 'kalidoku-*.dump.gpg' \
                -mtime "+${RETENTION_DAYS}" -print -delete | wc -l)
    log "retention prune older_than=${RETENTION_DAYS}d removed=${PRUNED}"
fi

log "ok"
