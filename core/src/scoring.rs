//! Originality score — rewards picking less famous entities.
//!
//! Each solved cell contributes `100 - fame_score` (entity with `fame_score = None`
//! is treated as 50, neutral). The total is normalised over a full 9-cell board
//! so the value is always in 0..=100.

use crate::entity::{Entity, FAME_MAX_PER_CELL};
// FAME_MAX_PER_CELL is the per-cell ceiling — kept as a public constant in
// entity.rs so external schemas can reference it.

/// Number of cells on a kalidoku board (3x3 fixed by design).
pub const BOARD_CELL_COUNT: u32 = 9;

/// Max theoretical raw originality on a full board: 9 cells × 100 points.
pub const ORIGINALITY_RAW_MAX: u32 = BOARD_CELL_COUNT * FAME_MAX_PER_CELL;

/// Adds a single solved cell to a running raw originality total.
///
/// Returns `100 - effective_fame(entity)` capped at 100.
#[must_use]
pub fn cell_contribution(entity: &Entity) -> u32 {
    u32::from(100u8 - entity.effective_fame())
}

/// Normalises a raw originality (`sum of cell contributions`) to 0..=100.
///
/// `raw` is clamped against the theoretical max so callers cannot overflow
/// by mistake (e.g. if a future variant adds bonus cells).
#[must_use]
pub fn normalise(raw: u32) -> u8 {
    let clamped = raw.min(ORIGINALITY_RAW_MAX);
    // ((clamped * 100) / ORIGINALITY_RAW_MAX) is in 0..=100 by construction.
    ((clamped * 100) / ORIGINALITY_RAW_MAX) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    use std::collections::HashMap;

    fn entity_with_fame(fame: Option<u8>) -> Entity {
        Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: HashMap::new(),
            fame_score: fame,
            icon_url: None,
        }
    }

    #[test]
    fn fame_none_contributes_50() {
        assert_eq!(cell_contribution(&entity_with_fame(None)), 50);
    }

    #[test]
    fn fame_zero_gives_full_100() {
        assert_eq!(cell_contribution(&entity_with_fame(Some(0))), 100);
    }

    #[test]
    fn fame_hundred_gives_zero() {
        assert_eq!(cell_contribution(&entity_with_fame(Some(100))), 0);
    }

    #[test]
    fn fame_above_hundred_is_clamped() {
        assert_eq!(cell_contribution(&entity_with_fame(Some(200))), 0);
    }

    #[test]
    fn normalise_full_board_neutral_is_50() {
        // 9 cells × 50 = 450 raw → 50/100.
        assert_eq!(normalise(450), 50);
    }

    #[test]
    fn normalise_max_is_100() {
        assert_eq!(normalise(ORIGINALITY_RAW_MAX), 100);
    }

    #[test]
    fn normalise_overflow_is_clamped() {
        assert_eq!(normalise(ORIGINALITY_RAW_MAX + 500), 100);
    }

    #[test]
    fn normalise_zero_is_zero() {
        assert_eq!(normalise(0), 0);
    }
}
