//! `starts_with(letter)` — true when the entity name starts with the given letter
//! (after `normalize`).

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

const FAMILY: &str = "starts_with";

#[derive(Debug)]
pub struct StartsWith {
    id: String,
    letter: String,
    labels: StoredLabels,
}

impl StartsWith {
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

impl Predicate for StartsWith {
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
        normalised_name(&entity.name).starts_with(&self.letter)
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let letter = param_as_letter(FAMILY, &def.param)?;
        Ok(Arc::new(StartsWith::new(letter, stored_labels(def))))
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
                text: "Commence par C".into(),
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
    fn matches_when_name_starts_with_letter() {
        let p = StartsWith::new('C', labels());
        assert!(p.matches(&ent("Châtelet")));
        assert!(p.matches(&ent("Concorde")));
    }

    #[test]
    fn rejects_when_first_letter_is_different() {
        let p = StartsWith::new('C', labels());
        assert!(!p.matches(&ent("Bastille")));
        assert!(!p.matches(&ent("Opéra")));
    }
}
