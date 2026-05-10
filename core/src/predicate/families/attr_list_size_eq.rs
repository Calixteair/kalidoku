//! `attr_list_size_eq(attr, n)` — true when the entity declares a `str_list`
//! attribute named `attr` whose cardinality equals `n` exactly.
//!
//! Typical use case: "stations that serve exactly one line" (`n = 1`) or "stations
//! at the intersection of exactly two lines" (`n = 2`). Complements
//! [`crate::predicate::families::attr_list_size_gte`] for the open-ended threshold.

use crate::{
    entity::{AttributeValue, Entity},
    error::Result,
    predicate::{
        families::common::{
            entity_attr, param_as_attr_and_count, predicate_id, stored_labels, StoredLabels,
        },
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "attr_list_size_eq";

#[derive(Debug)]
pub struct AttrListSizeEq {
    id: String,
    attr: String,
    target: usize,
    labels: StoredLabels,
}

impl AttrListSizeEq {
    #[must_use]
    pub fn new(attr: String, target: usize, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &format!("{attr}={target}")),
            attr,
            target,
            labels,
        }
    }
}

impl Predicate for AttrListSizeEq {
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
        matches!(
            entity_attr(entity, &self.attr),
            Some(AttributeValue::StrList(list)) if list.len() == self.target
        )
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let (attr, n) = param_as_attr_and_count(FAMILY, &def.param)?;
        Ok(Arc::new(AttrListSizeEq::new(attr, n, stored_labels(def))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::AttributeValue,
        predicate::{Label, PredicateLabels},
    };
    use std::collections::{BTreeMap, HashMap};

    fn labels() -> StoredLabels {
        StoredLabels::from(&PredicateLabels {
            fr: Label {
                text: "Exactement n lignes".into(),
                help: None,
            },
            en: None,
            other: BTreeMap::new(),
        })
    }

    fn entity_with_lines(lines: &[&str]) -> Entity {
        let mut a = HashMap::new();
        a.insert(
            "lines".into(),
            AttributeValue::StrList(lines.iter().map(|s| (*s).to_string()).collect()),
        );
        Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: a,
        }
    }

    #[test]
    fn matches_only_when_size_matches_exactly() {
        let p = AttrListSizeEq::new("lines".into(), 2, labels());
        assert!(p.matches(&entity_with_lines(&["1", "4"])));
        assert!(!p.matches(&entity_with_lines(&["1"])));
        assert!(!p.matches(&entity_with_lines(&["1", "4", "7"])));
    }

    #[test]
    fn rejects_when_attribute_is_not_a_str_list() {
        let p = AttrListSizeEq::new("lines".into(), 1, labels());
        let mut a = HashMap::new();
        a.insert("lines".into(), AttributeValue::Str("not-a-list".into()));
        let e = Entity {
            id: "y".into(),
            name: "Y".into(),
            aliases: vec![],
            attributes: a,
        };
        assert!(!p.matches(&e));
    }
}
