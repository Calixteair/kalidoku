//! `attr_list_size_gte(attr, n)` — true when the entity declares a `str_list`
//! attribute named `attr` whose number of elements is **at least** `n`.
//!
//! Typical use case for a transit dataset: "stations that serve at least 3 lines".
//! Mirrors [`crate::predicate::families::numeric_gte`] but operates on the cardinality
//! of a list attribute rather than on a numeric scalar.

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

const FAMILY: &str = "attr_list_size_gte";

#[derive(Debug)]
pub struct AttrListSizeGte {
    id: String,
    attr: String,
    threshold: usize,
    labels: StoredLabels,
}

impl AttrListSizeGte {
    #[must_use]
    pub fn new(attr: String, threshold: usize, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &format!("{attr}>={threshold}")),
            attr,
            threshold,
            labels,
        }
    }
}

impl Predicate for AttrListSizeGte {
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
            Some(AttributeValue::StrList(list)) if list.len() >= self.threshold
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
        Ok(Arc::new(AttrListSizeGte::new(attr, n, stored_labels(def))))
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
                text: "Au moins n lignes".into(),
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
    fn matches_when_size_at_or_above_threshold() {
        let p = AttrListSizeGte::new("lines".into(), 3, labels());
        assert!(p.matches(&entity_with_lines(&["1", "4", "7"])));
        assert!(p.matches(&entity_with_lines(&["1", "4", "7", "11"])));
    }

    #[test]
    fn rejects_when_size_below_threshold() {
        let p = AttrListSizeGte::new("lines".into(), 3, labels());
        assert!(!p.matches(&entity_with_lines(&["1", "4"])));
        assert!(!p.matches(&entity_with_lines(&[])));
    }

    #[test]
    fn rejects_when_attribute_missing_or_wrong_type() {
        let p = AttrListSizeGte::new("lines".into(), 1, labels());
        let e = Entity {
            id: "y".into(),
            name: "Y".into(),
            aliases: vec![],
            attributes: HashMap::new(),
        };
        assert!(!p.matches(&e));
    }

    #[test]
    fn id_includes_attr_and_threshold() {
        let p = AttrListSizeGte::new("lines".into(), 3, labels());
        assert_eq!(p.id(), "attr_list_size_gte:lines__3");
    }
}
