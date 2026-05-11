//! CSP-style generator. Picks 3 row predicates + 3 col predicates such that every
//! intersection has at least `min_candidates_per_cell` matching entities, every
//! intersection respects the `max_candidates_per_cell` upper bound, and a perfect
//! matching exists across the 9 cells (i.e. one assignment without entity reuse
//! is feasible).
//!
//! The matching check is a textbook augmenting-path search: at most 9 left-vertices,
//! so a simple recursive DFS is sufficient and easier to audit than a full
//! Hopcroft–Karp implementation.

use std::collections::{BTreeMap, HashSet};

use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use serde::Serialize;

use crate::{
    domain::Domain,
    entity::Entity,
    error::{Error, Result},
    predicate::DynPredicate,
};

/// 3x3 grid of clue intersections, accompanied by the entity ids that match each cell.
#[derive(Debug)]
pub struct Grid {
    pub rows: [DynPredicate; 3],
    pub cols: [DynPredicate; 3],
    /// `candidates[r][c]` lists entity ids that match both the row and the col predicate.
    pub candidates: [[Vec<String>; 3]; 3],
}

#[derive(Debug, Clone)]
pub struct GenerationOptions {
    pub seed: u64,
    pub max_attempts: u32,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            seed: rand::random(),
            max_attempts: 200,
        }
    }
}

/// Snapshot of [`Grid`] safe to serialise — hides the predicate trait objects
/// behind structured metadata.
#[derive(Debug, Serialize)]
pub struct GridSnapshot {
    pub seed: u64,
    pub rows: [PredicateSnapshot; 3],
    pub cols: [PredicateSnapshot; 3],
    pub candidates: [[Vec<String>; 3]; 3],
    /// Compact dictionary of every entity referenced in `candidates`, so the
    /// server can resolve user-typed names → entity_id without re-loading the
    /// domain pack on every request.
    pub entities: Vec<EntitySnapshot>,
}

#[derive(Debug, Serialize)]
pub struct PredicateSnapshot {
    pub id: String,
    pub family: String,
    pub label: String,
    pub help: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EntitySnapshot {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Snapshotted at generation time so the score is immutable — re-ingesting
    /// the domain with new fame scores does not retro-actively rewrite scores
    /// on past grids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fame_score: Option<u8>,
}

impl Grid {
    /// Render the grid for serialisation. Locale picks the label/help carried by each predicate.
    #[must_use]
    pub fn snapshot(&self, locale: &str, seed: u64) -> GridSnapshot {
        GridSnapshot {
            seed,
            rows: std::array::from_fn(|i| snap(&self.rows[i], locale)),
            cols: std::array::from_fn(|i| snap(&self.cols[i], locale)),
            candidates: self.candidates.clone(),
            entities: Vec::new(),
        }
    }

    /// Same as [`Grid::snapshot`] but also bakes in the dictionary of every entity
    /// referenced in `candidates`. Pass the full domain entity list — only the
    /// referenced ones are kept in the output.
    #[must_use]
    pub fn snapshot_with_entities(
        &self,
        locale: &str,
        seed: u64,
        all_entities: &[crate::entity::Entity],
    ) -> GridSnapshot {
        let mut used = std::collections::HashSet::new();
        for row in &self.candidates {
            for cell in row {
                for id in cell {
                    used.insert(id.clone());
                }
            }
        }
        let entities = all_entities
            .iter()
            .filter(|e| used.contains(&e.id))
            .map(|e| EntitySnapshot {
                id: e.id.clone(),
                name: e.name.clone(),
                aliases: e.aliases.clone(),
                fame_score: e.fame_score,
            })
            .collect();
        GridSnapshot {
            seed,
            rows: std::array::from_fn(|i| snap(&self.rows[i], locale)),
            cols: std::array::from_fn(|i| snap(&self.cols[i], locale)),
            candidates: self.candidates.clone(),
            entities,
        }
    }
}

fn snap(p: &DynPredicate, locale: &str) -> PredicateSnapshot {
    PredicateSnapshot {
        id: p.id().to_string(),
        family: p.family().to_string(),
        label: p.label(locale).to_string(),
        help: p.help(locale).map(str::to_string),
    }
}

/// Generate a 3x3 grid satisfying the domain's generator constraints.
pub fn generate(domain: &Domain, opts: GenerationOptions) -> Result<Grid> {
    let constraints = &domain.metadata.generator_constraints;
    if domain.predicates.len() < 6 {
        return Err(Error::InvalidDomain(format!(
            "domain '{}' needs at least 6 predicates, has {}",
            domain.metadata.id,
            domain.predicates.len()
        )));
    }

    let row_count = usize::try_from(constraints.row_predicate_count).unwrap_or(3);
    let col_count = usize::try_from(constraints.col_predicate_count).unwrap_or(3);
    if row_count != 3 || col_count != 3 {
        return Err(Error::InvalidDomain(format!(
            "this generator only supports 3x3 grids, got {row_count}x{col_count}"
        )));
    }

    let min_per_cell = usize::try_from(constraints.min_candidates_per_cell).unwrap_or(2);
    let max_per_cell = usize::try_from(constraints.max_candidates_per_cell).unwrap_or(usize::MAX);

    // Pre-compute matches[predicate_idx] = set of entity indices that satisfy it.
    let matches: Vec<Vec<usize>> = domain
        .predicates
        .iter()
        .map(|p| matching_entities(p.as_ref(), &domain.entities))
        .collect();

    let mut rng = StdRng::seed_from_u64(opts.seed);

    for _ in 0..opts.max_attempts {
        let mut order: Vec<usize> = (0..domain.predicates.len()).collect();
        order.shuffle(&mut rng);

        let Some((rows_idx, cols_idx)) = pick_predicates(domain, &order, constraints) else {
            continue;
        };

        // Build per-cell candidate sets.
        let mut cells: [[Vec<usize>; 3]; 3] =
            std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
        let mut ok = true;
        'cells: for r in 0..3 {
            for c in 0..3 {
                let inter = intersect_sorted(&matches[rows_idx[r]], &matches[cols_idx[c]]);
                let len = inter.len();
                if len < min_per_cell || len > max_per_cell {
                    ok = false;
                    break 'cells;
                }
                cells[r][c] = inter;
            }
        }
        if !ok {
            continue;
        }

        if !has_perfect_matching(&cells, domain.entities.len()) {
            continue;
        }

        let candidates: [[Vec<String>; 3]; 3] = std::array::from_fn(|r| {
            std::array::from_fn(|c| {
                cells[r][c]
                    .iter()
                    .map(|&i| domain.entities[i].id.clone())
                    .collect()
            })
        });

        let rows = std::array::from_fn(|i| domain.predicates[rows_idx[i]].clone());
        let cols = std::array::from_fn(|i| domain.predicates[cols_idx[i]].clone());

        return Ok(Grid {
            rows,
            cols,
            candidates,
        });
    }

    Err(Error::GeneratorExhausted {
        attempts: opts.max_attempts,
    })
}

fn matching_entities(
    predicate: &dyn crate::predicate::Predicate,
    entities: &[Entity],
) -> Vec<usize> {
    entities
        .iter()
        .enumerate()
        .filter_map(|(i, e)| if predicate.matches(e) { Some(i) } else { None })
        .collect()
}

/// Pick 3 row predicates and 3 col predicates respecting diversity rules:
/// - same family used at most `max_predicate_family_repeats` times **per axis**
///   (the same family may still appear on a row and a col, this is intentional —
///   otherwise small seed packs with 6 predicates and a repeated family could
///   never produce a grid).
/// - rows and cols must not share a predicate id.
fn pick_predicates(
    domain: &Domain,
    order: &[usize],
    constraints: &crate::domain::GeneratorConstraints,
) -> Option<([usize; 3], [usize; 3])> {
    let max_repeats = usize::try_from(constraints.max_predicate_family_repeats.max(1)).unwrap_or(1);
    let mut rows = [0usize; 3];
    let mut cols = [0usize; 3];
    let mut chosen: Vec<usize> = Vec::with_capacity(6);
    let mut row_family_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut col_family_count: BTreeMap<String, usize> = BTreeMap::new();

    for slot in 0..6 {
        let target = if slot < 3 {
            &mut row_family_count
        } else {
            &mut col_family_count
        };
        let mut picked = None;
        for &idx in order {
            if chosen.contains(&idx) {
                continue;
            }
            let fam = domain.predicates[idx].family();
            let cnt = target.get(fam).copied().unwrap_or(0);
            if cnt >= max_repeats {
                continue;
            }
            picked = Some((idx, fam.to_string()));
            break;
        }
        let (idx, fam) = picked?;
        chosen.push(idx);
        *target.entry(fam).or_insert(0) += 1;
        if slot < 3 {
            rows[slot] = idx;
        } else {
            cols[slot - 3] = idx;
        }
    }
    Some((rows, cols))
}

fn intersect_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
    let set_b: HashSet<usize> = b.iter().copied().collect();
    a.iter().copied().filter(|x| set_b.contains(x)).collect()
}

/// Test if the bipartite graph (cell, candidate-entity) admits a perfect matching
/// covering the 9 cells. Augmenting-path DFS — O(9 × E).
fn has_perfect_matching(cells: &[[Vec<usize>; 3]; 3], entity_count: usize) -> bool {
    let mut match_of_entity: Vec<Option<usize>> = vec![None; entity_count];
    for cell_idx in 0..9 {
        let mut seen = vec![false; entity_count];
        if !try_augment(cell_idx, cells, &mut match_of_entity, &mut seen) {
            return false;
        }
    }
    true
}

fn try_augment(
    cell_idx: usize,
    cells: &[[Vec<usize>; 3]; 3],
    match_of_entity: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    let candidates = &cells[cell_idx / 3][cell_idx % 3];
    for &ent in candidates {
        if seen[ent] {
            continue;
        }
        seen[ent] = true;
        let augmentable = match match_of_entity[ent] {
            None => true,
            Some(prev_cell) => try_augment(prev_cell, cells, match_of_entity, seen),
        };
        if augmentable {
            match_of_entity[ent] = Some(cell_idx);
            return true;
        }
    }
    false
}
