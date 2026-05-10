# Runbook — Rollback to a previous sha-X

## When to rollback

- Smoke test in `deploy.yml` fails after a CI deploy.
- Latency on `kalidoku.calixteair.fr/api/health` jumps post-deploy and bisect
  is not immediately obvious.
- New release introduces a 5xx burst (visible in NPM logs / Wazuh).
- Database migration succeeded but new code paths break in prod.

If the issue is on the **DB schema** (a migration ran), rollback is **not**
just a tag swap — go to §"Rolling back a destructive migration".

## Inputs you need

- Previous good commit short sha: `abc1234` (7 chars). Find via
  `gh run list --workflow=build-push.yml -L 5` or the Docker Hub tag list.
- SSH access (`vps-claude`).
- Confirmation that all three images for the target sha exist on Docker Hub:

```bash
for IMG in server worker web; do
    docker manifest inspect "calixteair/kalidoku-${IMG}:sha-abc1234" >/dev/null \
        && echo "kalidoku-${IMG}:sha-abc1234 ✓" \
        || echo "kalidoku-${IMG}:sha-abc1234 ✗ MISSING"
done
```

If any image is missing, **stop**: rolling back to a partial set is worse than
the current breakage. Build the missing image manually with
`gh workflow run build-push.yml -f ref=abc1234`.

## Rollback procedure

### 1. Pin the three images to the target sha

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

`bao-deploy.sh` re-renders the env file from OpenBao, calls
`docker compose --env-file ... up -d` with the pinned images, and waits for
healthchecks. Watchtower is intentionally not used — only manual deploys and
the CI pipeline pull tags.

### 2. Verify rollback

```bash
curl -fsS https://kalidoku.calixteair.fr/api/health | jq
# expected: { "status":"ok", "version":"<rolled-back version>", ... }

ssh vps-claude "sudo docker ps --filter name=kalidoku- --format 'table {{.Names}}\t{{.Image}}\t{{.Status}}'"
```

All three image columns should now show `sha-abc1234`.

### 3. Open a hotfix branch

Rolling back is a band-aid. To prevent the next CI deploy from re-rolling
forward to broken `latest`, create a `fix/<short-slug>` branch from the
broken main commit, push the actual fix, run CI, and merge. Until the fix
lands, set the GitHub branch protection rule "deploy on main" to **paused**
or temporarily disable the `deploy.yml` job.

## Rolling back a destructive migration

If `server/migrations/NNNN_*.sql` has destructive DDL (DROP COLUMN, narrow
type changes, etc.), the DB is now incompatible with the previous server
binary. Sequence:

1. `bao-deploy.sh kalidoku` with the rolled-back images **will** boot, but
   the migrator runs at startup. If a previous migration exists with the
   same number, this is a no-op; if missing, the previous server may attempt
   to re-write data with the old schema → corruption risk.
2. Restore the latest snapshot from `/home/calixteair/backups/kalidoku/`:

```bash
ssh vps-claude
sudo bash -c '
    set -euo pipefail
    LATEST=$(ls -t /home/calixteair/backups/kalidoku/kalidoku-*.dump.gpg | head -1)
    echo "restoring $LATEST"
    cd /home/calixteair/docker/projects/kalidoku
    docker compose --env-file /run/kalidoku/.env stop server worker
    gpg --batch --quiet --decrypt --passphrase "$PG_BACKUP_PASSPHRASE" "$LATEST" \
      | docker compose --env-file /run/kalidoku/.env exec -T postgres \
            pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner
    docker compose --env-file /run/kalidoku/.env start server worker
'
```

> The `PG_BACKUP_PASSPHRASE` must already be exported in the calling shell.
> Pull it from `/run/kalidoku/.env` only inside that ephemeral subshell:
> `set -a; . /run/kalidoku/.env; set +a; ...`. Do **not** pass it on the
> command line — `ps` would expose it.

3. Confirm row counts on critical tables (`SELECT count(*) FROM games;` etc.)
   match the snapshot timestamp before opening the gates back up.

## Audit trail

Every rollback is logged via `logger -t kalidoku-deploy ...` in
`bao-deploy.sh` and ends up in Wazuh. After rolling back, attach the
runbook execution to the corresponding GitHub issue or post-mortem.

## See also

- `docs/deploy.md` — full pipeline diagram
- `docs/runbooks/incident-bao-sealed.md` — recover OpenBao before any rollback
