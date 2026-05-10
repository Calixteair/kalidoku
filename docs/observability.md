# kalidoku — observability

Pragmatic, reuses the existing host-level stack (Wazuh agent + Prometheus +
sidecar postfix relay) already deployed on `calixteair.fr`. Nothing new to
deploy on the kalidoku side beyond log routing and a few alert rules.

## 1. Log pipeline

```
        kalidoku-* containers (json log driver)
                       │
                       ▼
   /var/lib/docker/containers/*/*.log   (host)
                       │
                       ▼
    wazuh-agent (host, ID 001 = vps-host)
                       │
                       ▼
    wazuh.manager       ──▶  wazuh.indexer  ──▶  wazuh.dashboard
                       │
                       ▼
              wazuh.smtp-relay (postfix sidecar)
                       │
                       ▼
              soc@calixteair.fr (Mailcow)
```

### How the host agent captures Docker stdout

The Docker `json-file` log driver writes one file per container under
`/var/lib/docker/containers/<id>/*.log`. Each line is a JSON object:

```json
{"log":"{\"level\":\"info\",\"target\":\"kalidoku::api\",\"msg\":\"...\"}\n",
 "stream":"stdout","time":"2026-05-10T03:14:15.926Z"}
```

The wazuh-agent on the host has a `<localfile>` block per container of
interest. Append the kalidoku entries to
`/var/ossec/etc/ossec.conf` (or to the agent-specific config block delivered
by the manager group) and reload the agent:

```xml
<!-- kalidoku-server: handler logs -->
<localfile>
  <log_format>json</log_format>
  <location>/var/lib/docker/containers/*/kalidoku-server-*.log</location>
  <label key="@source">docker.kalidoku-server</label>
</localfile>

<!-- kalidoku-worker: cron + grid generator -->
<localfile>
  <log_format>json</log_format>
  <location>/var/lib/docker/containers/*/kalidoku-worker-*.log</location>
  <label key="@source">docker.kalidoku-worker</label>
</localfile>

<!-- kalidoku-web: nginx access logs (already JSON via main_json format upstream) -->
<localfile>
  <log_format>json</log_format>
  <location>/var/lib/docker/containers/*/kalidoku-web-*.log</location>
  <label key="@source">docker.kalidoku-web</label>
</localfile>
```

Reload the agent:

```bash
ssh vps-claude "sudo systemctl restart wazuh-agent"
ssh vps-claude "sudo tail -n 5 /var/ossec/logs/ossec.log"
```

> The wildcard `kalidoku-server-*.log` matches Docker's per-container log
> rotation (`-json.log` and the rotated copies). The agent reads new files
> automatically.

### Tracing format on the backend side

The Rust backend uses `tracing` + `tracing-subscriber` JSON formatter. Every
line is one JSON document with at minimum:

```json
{"timestamp":"2026-05-10T03:14:15Z",
 "level":"INFO",
 "target":"kalidoku::api::play",
 "fields":{"message":"play_token validated","game_id":"...","device_id":"..."},
 "span":{"name":"POST /play","trace_id":"..."}}
```

Wazuh's JSON decoder ingests these natively (`<log_format>json</log_format>`),
exposing every field for rule matching (e.g. `data.fields.message`,
`data.level`).

## 2. Alert patterns

The patterns below land under
`/var/ossec/etc/rules/local_rules.d/kalidoku.xml` on the manager. Import via
the Wazuh dashboard "Management → Rules → Add" or push directly. After any
change, restart the manager:

```bash
ssh vps-claude "sudo docker compose -f /home/calixteair/docker/infra/soc/wazuh/compose.yaml exec wazuh.manager /var/ossec/bin/wazuh-control restart"
```

### 2.1 Backend ERROR rate

```xml
<group name="kalidoku,">

  <!-- Single ERROR — informational, level 5 -->
  <rule id="200001" level="5">
    <decoded_as>json</decoded_as>
    <field name="@source">^docker\.kalidoku-server$</field>
    <field name="level">^ERROR$</field>
    <description>kalidoku-server: ERROR-level log emitted</description>
    <group>kalidoku-server,errors,</group>
  </rule>

  <!-- Burst of 10 ERRORs in 60s — page SOC -->
  <rule id="200002" level="12" frequency="10" timeframe="60">
    <if_matched_sid>200001</if_matched_sid>
    <description>kalidoku-server: ERROR burst (10 in 60s) — investigate</description>
    <group>kalidoku-server,errors,burst,</group>
  </rule>

</group>
```

### 2.2 Auth + abuse signals

```xml
<group name="kalidoku,">

  <!-- play_token invalid: someone tampering with HMAC -->
  <rule id="200010" level="10">
    <decoded_as>json</decoded_as>
    <field name="@source">^docker\.kalidoku-server$</field>
    <field name="fields.message">^play_token (rejected|invalid|expired)$</field>
    <description>kalidoku: play_token validation failed</description>
    <group>kalidoku-server,anti-cheat,</group>
  </rule>

  <!-- Repeat: 30 invalid play_tokens in 5 min from same IP -->
  <rule id="200011" level="13" frequency="30" timeframe="300">
    <if_matched_sid>200010</if_matched_sid>
    <same_source_ip />
    <description>kalidoku: sustained play_token forging attempt from $(srcip)</description>
    <group>kalidoku-server,anti-cheat,attack,</group>
  </rule>

  <!-- tower_governor rate limit hit: bot/abuser noise -->
  <rule id="200020" level="6">
    <decoded_as>json</decoded_as>
    <field name="@source">^docker\.kalidoku-server$</field>
    <field name="fields.message">rate.?limit.*exceeded</field>
    <description>kalidoku: tower_governor rejected a request</description>
    <group>kalidoku-server,rate-limit,</group>
  </rule>

</group>
```

### 2.3 Web nginx 5xx

```xml
<group name="kalidoku,">

  <rule id="200030" level="7">
    <decoded_as>json</decoded_as>
    <field name="@source">^docker\.kalidoku-web$</field>
    <field name="status">^5\d\d$</field>
    <description>kalidoku-web: 5xx returned by nginx</description>
    <group>kalidoku-web,5xx,</group>
  </rule>

  <rule id="200031" level="11" frequency="20" timeframe="120">
    <if_matched_sid>200030</if_matched_sid>
    <description>kalidoku-web: 5xx burst — backend likely degraded</description>
    <group>kalidoku-web,5xx,burst,</group>
  </rule>

</group>
```

### 2.4 OpenBao + bao-agent reuse

The existing rules from `[[vps-chantier-bao-migration]] §Alerting BAO`
(`OpenBaoUnreachable`, `OpenBaoSealed`, `BaoAgentNotActive`,
`BaoAgentFlapping`) already cover the kalidoku stack — `bao-agent@kalidoku`
is matched by `name=~"bao-agent@.+\\.service"`. No new rule needed.

### 2.5 Severity → alert channel mapping

| Wazuh level | Channel | Latency target |
|---|---|---|
| ≥ 12 | mail `soc@calixteair.fr` (sidecar postfix → Mailcow:465) | < 30 s |
| 7–11 | dashboard only (review at next check-in) | n/a |
| ≤ 6 | dashboard only, kept 30 days | n/a |

## 3. Adding a mail alert via the existing postfix sidecar

The Wazuh `wazuh-maild` daemon is already configured against the in-network
`wazuh.smtp-relay` sidecar (boky/postfix). To enable mail on a new rule, set
`<options>alert_by_email</options>` inside the rule and ensure the rule's
level is high enough that `email_alerts` matches.

`/var/ossec/etc/ossec.conf` (manager) excerpt — already in place from Wazuh
phase 1, kept here as reference:

```xml
<global>
  <email_notification>yes</email_notification>
  <email_to>soc@calixteair.fr</email_to>
  <smtp_server>wazuh.smtp-relay</smtp_server>
  <email_from>wazuh@calixteair.fr</email_from>
  <email_maxperhour>20</email_maxperhour>
</global>

<!-- emit a mail for any kalidoku rule level >= 12 -->
<email_alerts>
  <email_to>soc@calixteair.fr</email_to>
  <group>kalidoku</group>
  <level>12</level>
</email_alerts>
```

Restart the manager after editing.

> **Charset reminder for SMTP creds**: `SMTP_PASSWORD` rendered by bao-agent
> for `wazuh.smtp-relay` must contain no `$` (Compose strips it). Other
> ASCII OK. Cf. `[[vps-wazuh-deployment-phase1]] §Mapping mots de passe`.

## 4. Smoke tests

### Backend ERROR burst → mail

```bash
# Trigger 10 forced 500s on a known endpoint that the test-mode of the
# backend exposes (cf. server feature `panic_on_path`). Server logs an ERROR
# per call, rule 200002 fires after 10 hits in 60s.
for i in $(seq 1 10); do
    curl -fsS -o /dev/null -w "%{http_code}\n" \
         https://kalidoku.calixteair.fr/api/__test__/force-500
done
```

A mail to `soc@calixteair.fr` should arrive within ~30 s with rule 200002.

### Bao agent stops → metric

Stop the agent for kalidoku for 6 minutes (longer than the rule's `for: 5m`):

```bash
ssh vps-claude "sudo systemctl stop bao-agent@kalidoku.service"
sleep 360
ssh vps-claude "sudo systemctl start bao-agent@kalidoku.service"
```

Expect the `BaoAgentNotActive{name="bao-agent@kalidoku.service"}` alert in
Prometheus / Alertmanager and a mail.

## 5. Dashboards

Kibana (Wazuh dashboard) saved searches to create:

- **kalidoku-server-errors** — `data.@source:"docker.kalidoku-server" AND data.level:"ERROR"`
- **kalidoku-anti-cheat** — `rule.id:200010 OR rule.id:200011`
- **kalidoku-web-5xx** — `data.@source:"docker.kalidoku-web" AND data.status:[500 TO 599]`

Pin them on the kalidoku-specific dashboard alongside the standard "Threat
Hunting" view. Add a single visual: `count() over time` grouped by
`rule.id` to spot bursts quickly.

## 6. Future (out of scope here)

- Push selected `tracing` events to Tempo/OTLP for distributed traces.
- Expose `/metrics` on the server (already on the roadmap, OAuth-protected
  for `prometheus-soc`) — once live, add SLO-based alerts:
  - `histogram_quantile(0.95, ...)` on `/api/play` < 250 ms.
  - 5xx ratio < 0.1% over 5 min.

## See also

- `docs/security.md` §10 — Monitoring + RGPD log retention
- `docs/runbooks/incident-bao-sealed.md` — what to do when the SOC alerts
- Wiki `[[vps-wazuh-deployment-phase1]]` — single source of truth for the
  Wazuh + sidecar postfix setup
