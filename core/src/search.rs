//! Linear scored autocomplete suitable for ≤ ~2k entities (metro, RER, Transilien).
//! For larger domains (airports, world rail) the server crate plugs MiniSearch via Node sidecar
//! or moves to a Rust full-text index — see ADR in docs/architecture.md.

use crate::{entity::Entity, normalize::normalize};

#[derive(Debug, Clone)]
pub struct Hit<'a> {
    pub entity: &'a Entity,
    pub score: u32,
}

#[must_use]
pub fn search<'a>(query: &str, entities: &'a [Entity], limit: usize) -> Vec<Hit<'a>> {
    let q = normalize(query);
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<Hit<'a>> = entities
        .iter()
        .filter_map(|e| {
            let n = normalize(&e.name);
            let mut score = 0u32;
            if n == q {
                score = 1000;
            } else if n.starts_with(&q) {
                score = 800;
            } else if n.split(' ').any(|w| w.starts_with(&q)) {
                score = 600;
            } else if n.contains(&q) {
                score = 400;
            } else if e.aliases.iter().any(|a| normalize(a).contains(&q)) {
                score = 200;
            }
            if score > 0 {
                Some(Hit { entity: e, score })
            } else {
                None
            }
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.entity.name.len().cmp(&b.entity.name.len()))
    });
    hits.truncate(limit);
    hits
}
