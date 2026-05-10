# Runbook — OpenBao SEALED after reboot

## TL;DR

After any reboot of the host VPS or restart of the `openbao` container,
OpenBao comes back **sealed**. Every `bao-agent@*.service` will fall into a
crashloop within minutes; `kalidoku.calixteair.fr` keeps running on whatever
env was loaded in RAM at the previous start, but the **next** `docker compose
up -d` (intentional or triggered by a CI deploy) will start with empty
environment variables and brick the stack.

Auto-unseal is intentionally not configured (cf. `[[lab-openbao]]` decision
log). Recovery is manual and takes ~5 minutes.

## Symptoms

| Where | What you see |
|---|---|
| `journalctl -u bao-agent@kalidoku.service` | repeated `error looking up wrapped secret ID: 503 sealed` or `wrapping token is not valid` |
| `https://secrets.calixteair.fr/v1/sys/health` | HTTP 503 (OpenBao up but sealed) — distinguishes from full DOWN (no response) |
| `/run/kalidoku/.env` | stale or empty (last render before the seal) |
| `kalidoku.calixteair.fr/api/health` | still 200 OK if containers were already running, **until** the next deploy |
| Wazuh / Prometheus | `OpenBaoSealed` alert firing on `soc@calixteair.fr` |

## Pre-flight checklist

You need:

- SSH access to the VPS (`vps-claude` alias).
- The 5 unseal keys from `~/.config/openbao/init-secrets.json` on the admin
  workstation. **Three of five** are required.
- `jq`, `curl`, `ssh` on your local machine. The VPS does **not** have the
  unseal keys; never store them there.

## Procedure

### 1. Verify the seal state

```bash
curl -fsS https://secrets.calixteair.fr/v1/sys/health -o - || true
# HTTP 200 → unsealed         (not the right runbook)
# HTTP 429 → standby          (not the right runbook)
# HTTP 503 → SEALED           (continue this runbook)
# no answer → DOWN container  (start the openbao container first)
```

If the response includes `"sealed":true`, proceed.

### 2. Read the unseal keys locally

```bash
KEYS_FILE="$HOME/.config/openbao/init-secrets.json"
test -r "$KEYS_FILE" || { echo "init-secrets.json missing — recover from offline GPG backup"; exit 1; }

# Take three random keys out of the five. Order doesn't matter.
mapfile -t KEYS < <(jq -r '.unseal_keys_b64[]' "$KEYS_FILE" | shuf | head -3)
```

### 3. Submit the three keys via HTTPS

```bash
for K in "${KEYS[@]}"; do
    curl -fsS -X POST \
         -d "{\"key\":\"${K}\"}" \
         https://secrets.calixteair.fr/v1/sys/unseal | jq -r '"sealed=\(.sealed) progress=\(.progress)/\(.t)"'
done
```

Expected last line: `sealed=false progress=0/3`. Confirm with
`curl -fsS https://secrets.calixteair.fr/v1/sys/health` returning HTTP 200.

### 4. Re-wrap each agent's secret_id

After a long seal window, every `bao-agent@*` has consumed its previous
wrap (or the wrap TTL has elapsed). Walk the active agents and refresh.

```bash
ROOT_TOKEN=$(jq -r .root_token ~/.config/openbao/init-secrets.json)

ssh vps-claude "sudo bash -c '
for SVC in \$(systemctl list-units \"bao-agent@*.service\" --all --no-legend --plain | awk \"{print \\\$1}\"); do
    STACK=\$(echo \$SVC | sed -E \"s/bao-agent@(.+)\\.service/\\1/\")
    ROLE=\$(grep -oE \"auth/approle/role/[^/]+\" /etc/bao/\$STACK/agent.hcl | head -1 | cut -d/ -f4)
    WRAP=\$(docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao write -wrap-ttl=10m -force -field=wrapping_token auth/approle/role/\$ROLE/secret-id)
    echo -n \"\$WRAP\" > /etc/bao/\$STACK/secret_id_wrap
    chmod 600 /etc/bao/\$STACK/secret_id_wrap
    systemctl stop \$SVC
done
sleep 2
for SVC in \$(systemctl list-units \"bao-agent@*.service\" --all --no-legend --plain | awk \"{print \\\$1}\"); do
    systemctl start \$SVC
done
'"
```

> **Critical**: stop **all** agents before regenerating their wraps. Using
> `systemctl restart` while another `bao-agent` is still running consumes the
> wrap before the file is even written. Always `stop` → `regenerate` →
> `start`.

### 5. Verify each agent rendered its file

```bash
ssh vps-claude "sudo bash -c '
for STACK in \$(ls /etc/bao); do
    [[ -d /etc/bao/\$STACK ]] || continue
    ENV_FILE=/run/\$STACK/.env
    if [[ -s \$ENV_FILE ]]; then
        echo \"\$STACK ✓ \$(stat -c %y \$ENV_FILE) (\$(wc -l < \$ENV_FILE) lines)\"
    else
        echo \"\$STACK ✗ MISSING: \$ENV_FILE\"
    fi
done'"
```

Each kalidoku-relevant line should show a recent timestamp. If `kalidoku ✗`,
re-check `journalctl -u bao-agent@kalidoku.service -n 30 --no-pager`.

### 6. Re-deploy kalidoku to refresh container env

The running containers still hold the *old* env in RAM. Force them to pick up
the freshly-rendered values:

```bash
ssh vps-claude "sudo /usr/local/sbin/bao-deploy.sh kalidoku"
```

### 7. Smoke test

```bash
curl -fsS https://kalidoku.calixteair.fr/api/health | jq
# {"status":"ok", "version":"...", "uptime_s":<small>}
```

## Aftermath

- If `prometheus` was itself crashlooped during the incident, alerting was
  blind. Verify it: `ssh vps-claude "docker ps --filter name=prometheus"` —
  if Exited, see `docs/runbooks/rollback.md` §"Prometheus down".
- File a post-mortem entry in `[[vps-chantier-bao-migration]] §Incident YYYY-MM-DD`
  if seal-time > 1h.
- Schedule a `bao-rotate-hmac.sh` run in the next maintenance window if you
  suspect any unseal key was exposed during recovery.

## See also

- `[[lab-openbao]]` — fundamentals + decision log on auto-unseal
- `[[vps-chantier-bao-migration]] §Incident 2026-05-03` — last comparable event
