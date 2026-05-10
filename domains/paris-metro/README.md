# Domain pack — paris-metro

Premier domain pack de référence pour kalidoku.

## Fichiers

- `metadata.json` — id, version, sources, contraintes générateur.
- `predicates.json` — 6 prédicats prêts à l'emploi (lettres, ligne, ligne automatique, géo).
- `entities.json` — 10 stations seed pour démarrer (à remplacer par l'ingestion complète).
- `scripts/ingest.ts` — pipeline d'ingestion GTFS + OSM + Wikidata (à implémenter par agent F).

## Couverture cible

- ~302 stations métro intra-muros + RER A/B intra-muros (ajout incrémental).
- Pour chaque station :
  - `name` canonique (FR), aliases (sans-accent, abréviations courantes).
  - `lines: str_list` — liste de lignes (`"1"`, `"7bis"`, `"RER A"`).
  - `geo: { lat, lon }` — précis à <100m via OSM.
  - `arrondissement: num` — 1 à 20, ou `0` si hors Paris.
  - `in_paris: bool`.
  - Attributs futurs : `opened_year`, `named_after`, `transfers_to`, `is_terminus`.

## Validation

```bash
# (agent F) script de validation contre contracts/entity-schema.json
pnpm dlx ajv-cli validate -s ../../contracts/entity-schema.json -d entities.json
```
