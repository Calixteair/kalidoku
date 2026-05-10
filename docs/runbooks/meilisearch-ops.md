# Runbook — Meilisearch ops (rotation master key, reindex)

Le service `kalidoku-search` expose Meilisearch v1.11 sur le réseau Docker
interne `kalidoku-internal`. Il n'est jamais routé via NPM. Seuls
`kalidoku-server` et `kalidoku-worker` s'y connectent via
`http://search:7700`.

## TL;DR endpoints

| Action | Path | Auth |
|---|---|---|
| Health | `GET /health` | aucune |
| Stats | `GET /stats` | `Authorization: Bearer $MEILI_MASTER_KEY` |
| Liste indexes | `GET /indexes` | master key |
| Stats par index | `GET /indexes/{uid}/stats` | master key |
| Tâches en cours | `GET /tasks?statuses=enqueued,processing` | master key |

## Rotation de la master key

Procédure annuelle ou en cas de suspicion de fuite. Coupure ~30 s.

### 1. Générer la nouvelle clé localement

```bash
NEW_KEY=$(openssl rand -base64 48 | tr -d '\n=' | head -c 64)
```

### 2. Patch dans OpenBao (jamais `kv put`, sinon les autres clés disparaissent)

```bash
ROOT_TOKEN=$(jq -r .root_token ~/.config/openbao/init-secrets.json)
ssh vps-claude "sudo docker exec -e BAO_TOKEN=$ROOT_TOKEN openbao bao kv patch \
    secret/kalidoku/prod/search MEILI_MASTER_KEY='$NEW_KEY'"
```

### 3. Re-render env + redeploy

`bao-agent` voit le nouveau secret au refresh suivant (TTL court). Pour
forcer un render immédiat puis recréer les containers concernés :

```bash
ssh vps-claude "sudo /usr/local/sbin/bao-deploy.sh kalidoku"
```

`docker compose up -d` ne recrée que les services dont la conf a changé.
Avec la nouvelle valeur de `MEILI_MASTER_KEY`, les trois containers
`search`, `server` et `worker` sont recréés simultanément — pas de
fenêtre d'incohérence où un client utiliserait l'ancienne clé contre une
nouvelle instance Meili (ou inversement).

> Meilisearch persiste l'ancienne master key dans `kalidoku_meili` sous
> forme dérivée. Au boot, si la valeur de `MEILI_MASTER_KEY` ne
> correspond pas, **Meilisearch refuse de démarrer**. Si la rotation a
> été partielle (par exemple Bao patché mais containers pas recréés),
> stop le container `search` puis supprime `auth/` dans le volume :
>
> ```bash
> ssh vps-claude "sudo docker compose -f /home/calixteair/docker/projects/kalidoku/compose.yaml stop search"
> ssh vps-claude "sudo docker run --rm -v kalidoku_kalidoku_meili:/data alpine sh -c 'rm -rf /data/auth /data/instance-uid'"
> ssh vps-claude "sudo /usr/local/sbin/bao-deploy.sh kalidoku"
> ```
>
> Cette opération invalide les API keys dérivées (jamais utilisées par
> kalidoku — on ne sert que via la master key côté backend), mais
> conserve les indexes.

### 4. Smoke test

```bash
ssh vps-claude "sudo docker exec kalidoku-server \
    wget -qO- --header=\"Authorization: Bearer \$MEILI_MASTER_KEY\" \
    http://search:7700/stats"
```

Doit retourner du JSON avec `databaseSize` non nul.

## Reindexation manuelle

Cas d'usage :

- Schéma d'un domain pack a changé (nouvel attribut).
- Volume Meili corrompu après crash disque.
- Drift suspecté entre Postgres et l'index.

Le worker expose un mode `--reindex <domain>` qui purge l'index puis
réinjecte tout depuis Postgres. Idempotent.

```bash
ssh vps-claude "sudo docker compose -f /home/calixteair/docker/projects/kalidoku/compose.yaml \
    --env-file /run/kalidoku/.env \
    run --rm worker --reindex paris-metro"
```

Pour reindexer **tous** les domaines :

```bash
ssh vps-claude "sudo docker compose -f /home/calixteair/docker/projects/kalidoku/compose.yaml \
    --env-file /run/kalidoku/.env \
    run --rm worker --reindex-all"
```

Suivre la tâche côté Meili :

```bash
ssh vps-claude "sudo docker exec kalidoku-server \
    wget -qO- --header=\"Authorization: Bearer \$MEILI_MASTER_KEY\" \
    'http://search:7700/tasks?statuses=enqueued,processing&limit=10'"
```

## Reset complet du volume

Dernier recours. Toutes les données d'index disparaissent — la
reindexation depuis Postgres prend ~2 min pour `paris-metro`, ~10 min
pour `world-airports`.

```bash
ssh vps-claude "sudo docker compose -f /home/calixteair/docker/projects/kalidoku/compose.yaml stop search"
ssh vps-claude "sudo docker volume rm kalidoku_kalidoku_meili"
ssh vps-claude "sudo /usr/local/sbin/bao-deploy.sh kalidoku"
ssh vps-claude "sudo docker compose -f /home/calixteair/docker/projects/kalidoku/compose.yaml \
    --env-file /run/kalidoku/.env run --rm worker --reindex-all"
```

## Capacité

- Mémoire : `MEILI_MAX_INDEXING_MEMORY=256Mb` côté env, `mem_limit: 384m`
  côté compose. À surveiller : si la roadmap dépasse 100k docs sur un
  même domaine, bump à 512 MB.
- Disque : volume `kalidoku_meili` ~50 MB pour 12k docs aux estimations
  Phase 3. Pas de rotation nécessaire côté volume.
- Pas de réplication, pas de backup dédié — l'index est dérivable depuis
  Postgres (source de vérité), donc une perte volume est récupérable via
  `--reindex-all`.

## Voir aussi

- `infra/docker/compose.yaml` — service `search`
- `infra/bao/agent.hcl` — render `MEILI_MASTER_KEY`
- `docs/architecture.md` ADR-006 — choix Meilisearch vs MiniSearch
