//! `name_word_count_max(n)` — true when the entity's name has at most `n` words.

use crate::{
    entity::Entity,
    error::Result,
    predicate::{
        families::common::{count_words, param_as_u32, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "name_word_count_max";

#[derive(Debug)]
pub struct NameWordCountMax {
    id: String,
    max: u32,
    labels: StoredLabels,
}

impl NameWordCountMax {
    #[must_use]
    pub fn new(max: u32, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &max.to_string()),
            max,
            labels,
        }
    }
}

impl Predicate for NameWordCountMax {
    fn id(&self) -> &str {
        &self.id
    }
    fn family(&self) -> &str {
        FAMILY
    }
    fn label(&self, locale: &str) -> &str {
        &self.labels.pick(locale).text
    }
    fn help(&self, locale: &str) -> Option<&str> {
        self.labels.pick(locale).help.as_deref()
    }
    fn matches(&self, entity: &Entity) -> bool {
        count_words(&entity.name) <= self.max
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let max = param_as_u32(FAMILY, &def.param)?;
        Ok(Arc::new(NameWordCountMax::new(max, stored_labels(def))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicate::{Label, PredicateLabels};
    use std::collections::BTreeMap;

    fn labels() -> StoredLabels {
        StoredLabels::from(&PredicateLabels {
            fr: Label {
                text: "Mots-".into(),
                help: None,
            },
            en: None,
            other: BTreeMap::new(),
        })
    }

    fn ent(name: &str) -> Entity {
        Entity {
            id: name.to_lowercase(),
            name: name.into(),
            aliases: vec![],
            attributes: std::collections::HashMap::new(),
            fame_score: None,
        }
    }

    #[test]
    fn matches_at_most() {
        let p = NameWordCountMax::new(2, labels());
        assert!(p.matches(&ent("Châtelet"))); // 1
        assert!(p.matches(&ent("Saint-Lazare"))); // 2
    }

    #[test]
    fn rejects_above() {
        let p = NameWordCountMax::new(2, labels());
        assert!(!p.matches(&ent("Charles de Gaulle — Étoile"))); // 4
    }
}
