# Agent A — `core/`

## Périmètre

Tout `core/` et son binaire CLI `kalidoku-generate`.

## Livrables

1. **Implémentation des familles de prédicats** (`core/src/predicate/families/`):
   - `ends_with(letter)`
   - `starts_with(letter)`
   - `contains_letter(letter)`
   - `name_length_max(n)`, `name_length_min(n)`, `name_length_eq(n)`
   - `on_attr_eq(attr, value)`
   - `on_attr_in_set(attr, values)`
   - `on_attr_contains(attr, value)`  (pour `str_list`)
   - `within_km(attr, point, km)`
   - `numeric_gte(attr, n)`, `numeric_lte`, `numeric_between`

   Chaque famille = un struct + un `PredicateFactory` enregistré dans `PredicateRegistry::with_defaults()`.

2. **Loader de domain pack** (`core/src/domain/loader.rs`) :
   ```rust
   pub fn load_domain_pack(dir: &Path) -> Result<Domain> { ... }
   ```
   Lit `metadata.json`, `predicates.json`, `entities.json`, valide contre `contracts/entity-schema.json`.

3. **Générateur CSP** (`core/src/generator.rs`) :
   - Tire 6 prédicats avec règles de diversité (`max_predicate_family_repeats`, mix tags).
   - Calcule `S_ij` pour chaque cellule (3×3).
   - Vérifie `min_candidates_per_cell ≤ |S_ij| ≤ max_candidates_per_cell`.
   - Vérifie l'existence d'un perfect matching (Hopcroft-Karp sur biparti cellule↔entité).
   - Backtrack jusqu'à `max_attempts`.

4. **CLI** `core/src/bin/generate.rs` :
   ```bash
   kalidoku-generate --domain ./domains/paris-metro --seed 2026-05-10 --pretty > grid.json
   ```

5. **Tests** :
   - Unitaires sur chaque famille de prédicats (10 stations seed du domain `paris-metro`).
   - Intégration `generate` → grille valide pour les 10 stations seed.
   - Coverage > 80 % sur `core/`.

## Contrainte forte

- `core/` reste **synchrone, sans I/O réseau, sans dépendance async**. Le seul I/O autorisé est `serde_json` pour les domain packs (via le loader).
- API publique stable (cf. signatures dans `core/src/lib.rs` et `generator.rs`). Toute cassure = PR de contrat.

## Acceptance

- [ ] `cargo build -p kalidoku-core` passe.
- [ ] `cargo test -p kalidoku-core` passe (tous les nouveaux tests).
- [ ] `cargo clippy -p kalidoku-core --all-targets -- -D warnings` clean.
- [ ] `cargo run --bin kalidoku-generate -- --domain domains/paris-metro --seed 1` produit une grille valide en JSON.
- [ ] Doc des familles dans `core/src/predicate/families/README.md`.

## Non-objectifs

- Pas d'intégration HTTP (c'est `server/`).
- Pas de chargement DB (c'est `worker/` ou `server/`).
- Pas d'i18n côté code (les labels sont des strings dans le domain pack).
