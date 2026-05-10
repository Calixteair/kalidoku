//! `name_word_count_eq(n)` — true when the entity's name has exactly `n` words,
//! with apostrophes kept inside a word (so "L'Haÿ-les-Roses" → 3 words).

use crate::{
    entity::Entity,
    error::Result,
    predicate::{
        families::common::{count_words, param_as_u32, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "name_word_count_eq";

#[derive(Debug)]
pub struct NameWordCountEq {
    id: String,
    n: u32,
    labels: StoredLabels,
}

impl NameWordCountEq {
    #[must_use]
    pub fn new(n: u32, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &n.to_string()),
            n,
            labels,
        }
    }
}

impl Predicate for NameWordCountEq {
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
        count_words(&entity.name) == self.n
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let n = param_as_u32(FAMILY, &def.param)?;
        Ok(Arc::new(NameWordCountEq::new(n, stored_labels(def))))
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
                text: "Mots".into(),
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
        }
    }

    #[test]
    fn matches_single_word() {
        let p = NameWordCountEq::new(1, labels());
        assert!(p.matches(&ent("Châtelet")));
        assert!(p.matches(&ent("Concorde")));
    }

    #[test]
    fn matches_multi_word() {
        let p = NameWordCountEq::new(2, labels());
        assert!(p.matches(&ent("Saint-Lazare")));
        assert!(p.matches(&ent("Place Monge")));
        let p4 = NameWordCountEq::new(4, labels());
        assert!(p4.matches(&ent("Charles de Gaulle — Étoile")));
    }

    #[test]
    fn apostrophe_is_part_of_word() {
        let p = NameWordCountEq::new(3, labels());
        assert!(p.matches(&ent("L'Haÿ-les-Roses"))); // L'Haÿ / les / Roses
    }
}
