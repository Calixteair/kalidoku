//! `contains_letter(letter)` — true when the entity name contains the given letter
//! (after `normalize`). Spaces inside the normalised name are not part of the search
//! space — but a letter that happens to coincide with the space character is rejected
//! at param parsing.

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

const FAMILY: &str = "contains_letter";

#[derive(Debug)]
pub struct ContainsLetter {
    id: String,
    letter: String,
    labels: StoredLabels,
}

impl ContainsLetter {
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

impl Predicate for ContainsLetter {
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
        normalised_name(&entity.name).contains(&self.letter)
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let letter = param_as_letter(FAMILY, &def.param)?;
        Ok(Arc::new(ContainsLetter::new(letter, stored_labels(def))))
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
                text: "Contient C".into(),
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
    fn matches_when_name_contains_letter() {
        let p = ContainsLetter::new('C', labels());
        assert!(p.matches(&ent("Châtelet")));
        assert!(p.matches(&ent("Concorde")));
        assert!(p.matches(&ent("Saint-Michel")));
    }

    #[test]
    fn does_not_match_when_letter_absent_in_normalised_name() {
        let p = ContainsLetter::new('c', labels());
        assert!(!p.matches(&ent("Bastille")));
    }

    #[test]
    fn rejects_when_name_does_not_contain_letter() {
        let p = ContainsLetter::new('z', labels());
        assert!(!p.matches(&ent("Bastille")));
        assert!(!p.matches(&ent("Opéra")));
    }

    #[test]
    fn diacritic_letter_normalized() {
        let p = ContainsLetter::new('e', labels());
        assert!(p.matches(&ent("Opéra")));
    }
}
