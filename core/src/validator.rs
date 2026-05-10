use crate::{
    domain::Domain,
    generator::Grid,
    normalize::normalize,
};

/// Result of validating a single cell answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellValidation {
    Match,
    Wrong,
    UnknownEntity,
    AlreadyUsed,
}

#[must_use]
pub fn validate_answer(
    domain: &Domain,
    grid: &Grid,
    used_entity_ids: &[String],
    cell: (usize, usize),
    raw_answer: &str,
) -> CellValidation {
    let normalised = normalize(raw_answer);
    let Some(entity) = domain.entities.iter().find(|e| {
        normalize(&e.name) == normalised
            || e.aliases.iter().any(|a| normalize(a) == normalised)
    }) else {
        return CellValidation::UnknownEntity;
    };
    if used_entity_ids.contains(&entity.id) {
        return CellValidation::AlreadyUsed;
    }
    let (r, c) = cell;
    if grid.candidates[r][c].iter().any(|id| id == &entity.id) {
        CellValidation::Match
    } else {
        CellValidation::Wrong
    }
}
