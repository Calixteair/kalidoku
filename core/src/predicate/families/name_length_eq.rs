//! `name_length_eq(n)` — true when the entity's normalised name has exactly `n`
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

const FAMILY: &str = "name_length_eq";

#[derive(Debug)]
pub struct NameLengthEq {
    id: String,
    n: u32,
    labels: StoredLabels,
}

impl NameLengthEq {
    #[must_use]
    pub fn new(n: u32, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &n.to_string()),
            n,
            labels,
        }
    }
}

impl Predicate for NameLengthEq {
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
        count_letters(&entity.name) == self.n
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let n = param_as_u32(FAMILY, &def.param)?;
        Ok(Arc::new(NameLengthEq::new(n, stored_labels(def))))
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
                text: "Exact".into(),
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
    fn matches_exact_length() {
        let p = NameLengthEq::new(6, labels());
        assert!(p.matches(&ent("Nation"))); // 6
    }

    #[test]
    fn rejects_other_lengths() {
        let p = NameLengthEq::new(6, labels());
        assert!(!p.matches(&ent("Concorde")));
        assert!(!p.matches(&ent("Auber")));
    }
}
