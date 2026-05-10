//! `name_length_max(n)` — true when the entity's normalised name (without spaces)
//! has at most `n` characters.

use crate::{
    entity::Entity,
    error::Result,
    predicate::{
        families::common::{
            normalised_name, param_as_u32, predicate_id, stored_labels, StoredLabels,
        },
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "name_length_max";

#[derive(Debug)]
pub struct NameLengthMax {
    id: String,
    max: u32,
    labels: StoredLabels,
}

impl NameLengthMax {
    #[must_use]
    pub fn new(max: u32, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &max.to_string()),
            max,
            labels,
        }
    }
}

#[must_use]
pub fn count_letters(name: &str) -> u32 {
    let n = normalised_name(name);
    u32::try_from(n.chars().filter(|c| !c.is_whitespace()).count()).unwrap_or(u32::MAX)
}

impl Predicate for NameLengthMax {
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
        count_letters(&entity.name) <= self.max
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let max = param_as_u32(FAMILY, &def.param)?;
        Ok(Arc::new(NameLengthMax::new(max, stored_labels(def))))
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
                text: "Court".into(),
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
    fn matches_short_names() {
        let p = NameLengthMax::new(7, labels());
        assert!(p.matches(&ent("Auber"))); // 5
        assert!(p.matches(&ent("Nation"))); // 6
        assert!(p.matches(&ent("Opéra"))); // 5 (after normalize)
    }

    #[test]
    fn rejects_long_names() {
        let p = NameLengthMax::new(7, labels());
        assert!(!p.matches(&ent("Concorde"))); // 8
        assert!(!p.matches(&ent("Saint-Michel"))); // 11 (no spaces)
    }
}
