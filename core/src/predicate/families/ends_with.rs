//! `ends_with(letter)` — true when the entity name ends with the given letter
//! (after `normalize`, which strips diacritics and lowercases).

use crate::{
    entity::Entity,
    error::Result,
    predicate::{
        families::common::{
            normalised_letter, normalised_name, param_as_letter, predicate_id, stored_labels,
            StoredLabels,
        },
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "ends_with";

#[derive(Debug)]
pub struct EndsWith {
    id: String,
    letter: String,
    labels: StoredLabels,
}

impl EndsWith {
    #[must_use]
    pub fn new(letter: char, labels: StoredLabels) -> Self {
        let normalised = normalised_letter(letter);
        let id = predicate_id(FAMILY, &normalised);
        Self {
            id,
            letter: normalised,
            labels,
        }
    }
}

impl Predicate for EndsWith {
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
        normalised_name(&entity.name).ends_with(&self.letter)
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let letter = param_as_letter(FAMILY, &def.param)?;
        Ok(Arc::new(EndsWith::new(letter, stored_labels(def))))
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
                text: "Termine par S".into(),
                help: None,
            },
            en: Some(Label {
                text: "Ends with S".into(),
                help: None,
            }),
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
    fn matches_when_name_ends_with_letter() {
        let p = EndsWith::new('S', labels());
        assert!(p.matches(&ent("Halles")));
        assert!(p.matches(&ent("Châtelet-Les Halles")));
    }

    #[test]
    fn rejects_when_name_does_not_end_with_letter() {
        let p = EndsWith::new('S', labels());
        assert!(!p.matches(&ent("Bastille")));
        assert!(!p.matches(&ent("Auber")));
    }

    #[test]
    fn case_insensitive_via_normalize() {
        let p = EndsWith::new('e', labels());
        assert!(p.matches(&ent("Bastille")));
        assert!(!p.matches(&ent("Auber")));
    }

    #[test]
    fn diacritics_are_stripped() {
        let p = EndsWith::new('a', labels());
        assert!(p.matches(&ent("Opéra")));
    }

    #[test]
    fn id_is_stable() {
        let p = EndsWith::new('S', labels());
        assert_eq!(p.id(), "ends_with:s");
    }
}
