# Agent F — `domains/paris-metro/` (datasets)

## Périmètre

`domains/paris-metro/` et le pipeline d'ingestion. **Pas de Rust.** Uniquement TypeScript/Python pour l'ingestion + JSON résultant.

## Livrables

### 1. Pipeline d'ingestion `scripts/ingest.ts`

Source #1 — **GTFS IDFM** :
- Téléchargement automatique depuis `https://prim.iledefrance-mobilites.fr/...` (token gratuit requis).
- Filtre `route_type ∈ {1 = subway}` pour ne garder que le métro.
- Joins `stops.txt` + `routes.txt` + `trips.txt` + `stop_times.txt` pour calculer `lines: str_list`.
- Géo : centroïde des `parent_station` quand dispo.

Source #2 — **OSM Overpass** :
- Query Overpass : `[out:json]; area["name"="Paris"]; node(area)[railway=station][station=subway]; out;`.
- Cache local en `tmp/osm-overpass.json`.
- Reconciliation par nom + proximité géo (<150 m) avec GTFS.
- Ajoute `wikipedia` tag, `wikidata` tag si présents.

Source #3 — **Wikidata SPARQL** :
- Query : `SELECT ?station ?stationLabel ?openedDate ?namedAfter WHERE { ?station wdt:P31 wd:Q928830 . ?station wdt:P276 wd:Q90 . ... }`
- Cache `tmp/wikidata.json`.
- Ajoute `opened_year`, `named_after_id`, `named_after_label`.

### 2. Output

- `entities.json` : trié par `id`, ~302 stations.
- Validation contre `contracts/entity-schema.json` via `ajv-cli` à la fin.
- Hash SHA-256 du résultat dans `metadata.json` `dataset_sha256` pour détecter changements.

### 3. Tests de qualité

- Au moins 290 stations (couvrir le métro intra-muros).
- 100% des entités ont `lines`, `geo`, `arrondissement`, `in_paris`.
- Pas de doublon `id` ni `name`.

### 4. Reproductibilité

- Sources versionnées : URL + date + sha256 dans `metadata.json` → `sources_versions[]`.
- Le pipeline ne touche pas au réseau si les caches `tmp/*.json` existent et sont récents (< 7 jours).

### 5. Évolution multi-domaine

Documenter dans `domains/_TEMPLATE.md` (à créer) la procédure pour ajouter un nouveau domain pack :
- Choisir un id en `kebab-case`.
- Lister les attributs nécessaires aux prédicats voulus.
- Trouver les sources libres.
- Écrire `metadata.json`, `predicates.json`, `entities.json`.
- Valider contre `contracts/entity-schema.json`.

## Acceptance

- [ ] `pnpm tsx domains/paris-metro/scripts/ingest.ts` produit un `entities.json` valide vs schema.
- [ ] Le générateur (`kalidoku-generate --domain ./domains/paris-metro`) produit une grille valide pour 100 seeds différents sans échec.
- [ ] README à jour côté `domains/paris-metro/`.
