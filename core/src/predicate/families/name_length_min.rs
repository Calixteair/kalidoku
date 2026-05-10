//! `name_length_min(n)` — true when the entity's normalised name has at least `n`
//! non-whitespace characters.

use super::{
    common::{param_as_u32, predicate_id, stored_labels, StoredLabels},
    name_length_max::count_letters,
};
use crate::{
    entity::Entity,
    error::Result,
    predicate::{DynPredicate, Predicate, PredicateDefinition, PredicateFactory},
};
use std::sync::Arc;

const FAMILY: &str = "name_length_min";

#[derive(Debug)]
pub struct NameLengthMin {
    id: String,
    min: u32,
    labels: StoredLabels,
}

impl NameLengthMin {
    #[must_use]
    pub fn new(min: u32, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &min.to_string()),
            min,
            labels,
        }
    }
}

impl Predicate for NameLengthMin {
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
        count_letters(&entity.name) >= self.min
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let min = param_as_u32(FAMILY, &def.param)?;
        Ok(Arc::new(NameLengthMin::new(min, stored_labels(def))))
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
                text: "Long".into(),
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
    fn matches_long_names() {
        let p = NameLengthMin::new(8, labels());
        assert!(p.matches(&ent("Concorde"))); // 8
        assert!(p.matches(&ent("République"))); // 10
    }

    #[test]
    fn rejects_short_names() {
        let p = NameLengthMin::new(8, labels());
        assert!(!p.matches(&ent("Auber")));
        assert!(!p.matches(&ent("Nation")));
    }
}
