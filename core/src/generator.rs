//! CSP-style generator. Picks 3 row predicates + 3 col predicates such that every
//! intersection has at least `min_candidates_per_cell` matching entities and a perfect
//! matching exists across the 9 cells (i.e. one assignment without reuse is feasible).
//!
//! Implementation lives behind a stable signature; agent A owns the internals.

use crate::{
    domain::Domain,
    error::Result,
    predicate::DynPredicate,
};

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

#[allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]
pub fn generate(_domain: &Domain, _opts: GenerationOptions) -> Result<Grid> {
    // TODO(agent-A): implement.
    // 1. pick 6 predicates respecting diversity (max_predicate_family_repeats)
    // 2. compute S_ij for all 9 cells
    // 3. validate every cell has |S_ij| ∈ [min, max]
    // 4. confirm a perfect matching exists (Hopcroft–Karp on 9 cells × entities bipartite)
    // 5. backtrack until success or attempts exhausted
    unimplemented!("agent-A: implement generator (see docs/agents/agent-a-core.md §4)")
}
