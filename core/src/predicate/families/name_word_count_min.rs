//! `name_word_count_min(n)` — true when the entity's name has at least `n` words.

use crate::{
    entity::Entity,
    error::Result,
    predicate::{
        families::common::{count_words, param_as_u32, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "name_word_count_min";

#[derive(Debug)]
pub struct NameWordCountMin {
    id: String,
    min: u32,
    labels: StoredLabels,
}

impl NameWordCountMin {
    #[must_use]
    pub fn new(min: u32, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &min.to_string()),
            min,
            labels,
        }
    }
}

impl Predicate for NameWordCountMin {
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
        count_words(&entity.name) >= self.min
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let min = param_as_u32(FAMILY, &def.param)?;
        Ok(Arc::new(NameWordCountMin::new(min, stored_labels(def))))
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
                text: "Mots+".into(),
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
            icon_url: None,
        }
    }

    #[test]
    fn matches_at_least() {
        let p = NameWordCountMin::new(3, labels());
        assert!(p.matches(&ent("Charles de Gaulle — Étoile"))); // 4
        assert!(p.matches(&ent("Place des Fêtes"))); // 3
    }

    #[test]
    fn rejects_below() {
        let p = NameWordCountMin::new(3, labels());
        assert!(!p.matches(&ent("Châtelet"))); // 1
        assert!(!p.matches(&ent("Saint-Lazare"))); // 2
    }
}
