# Agent B — `worker/`

## Périmètre

Tout `worker/`.

## Livrables

1. **Mode `--cron`** :
   - Démarre `tokio-cron-scheduler`.
   - Job quotidien à `00:01 UTC` qui itère sur les domaines actifs (table `domains` ou config) et appelle la même logique que `--once`.

2. **Mode `--once`** :
   - Pour chaque domaine actif, vérifie si la grille du jour existe (`SELECT … FROM grids WHERE (domain, mode, publish_at::date) = …`).
   - Si non, charge le domain pack via `core::domain::loader`, appelle `core::generator::generate` (seed dérivé de `(domain, date)` pour reproductibilité), persiste via SeaORM dans `grids`.

3. **Mode `--queue`** (phase MVP+) :
   - Worker queue Redis (`BLPOP solo:queue`), génère une grille solo à la demande, INSERT, push résultat sur `solo:done:<id>`.
   - Le `server/` enqueue les demandes solo et lit le résultat.

4. **Idempotence** :
   - `INSERT ... ON CONFLICT DO NOTHING` sur `(domain, mode, publish_at)`.
   - Si la table contient déjà la grille → log warn, skip.

5. **Tests** :
   - Test d'intégration : DB éphémère, run `--once` deux fois, vérifier qu'une seule ligne est créée.
   - Mock du domain pack via `tempfile`.

## Acceptance

- [ ] `cargo build -p kalidoku-worker` passe.
- [ ] `cargo run --bin kalidoku-worker -- --once` génère une grille pour `paris-metro`.
- [ ] Image Docker `kalidoku-worker:latest` build et démarre `--cron` sans crash.
