//! `numeric_gte(attr, n)` — true when the entity declares numeric attribute `attr`
//! with a value ≥ `n`.

use crate::{
    entity::{AttributeValue, Entity},
    error::Result,
    predicate::{
        families::common::{entity_attr, invalid_param, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "numeric_gte";

#[derive(Debug)]
pub struct NumericGte {
    id: String,
    attr: String,
    threshold: f64,
    labels: StoredLabels,
}

impl NumericGte {
    #[must_use]
    pub fn new(attr: String, threshold: f64, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &format!("{attr}>={threshold}")),
            attr,
            threshold,
            labels,
        }
    }
}

impl Predicate for NumericGte {
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
        matches!(entity_attr(entity, &self.attr), Some(AttributeValue::Num(n)) if *n >= self.threshold)
    }
}

pub struct Factory;

impl PredicateFactory for Factory {
    fn family(&self) -> &'static str {
        FAMILY
    }
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        let obj = def
            .param
            .as_object()
            .ok_or_else(|| invalid_param(FAMILY, "expected object {attr, n}"))?;
        let attr = obj
            .get("attr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_param(FAMILY, "missing string 'attr'"))?
            .to_string();
        let threshold = obj
            .get("n")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| invalid_param(FAMILY, "missing number 'n'"))?;
        Ok(Arc::new(NumericGte::new(
            attr,
            threshold,
            stored_labels(def),
        )))
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
                text: "≥".into(),
                help: None,
            },
            en: None,
            other: BTreeMap::new(),
        })
    }

    fn ent(arr: f64) -> Entity {
        let mut a = HashMap::new();
        a.insert("arrondissement".into(), AttributeValue::Num(arr));
        Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: a,
            fame_score: None,
        }
    }

    #[test]
    fn matches_when_value_above_threshold() {
        let p = NumericGte::new("arrondissement".into(), 5.0, labels());
        assert!(p.matches(&ent(11.0)));
        assert!(p.matches(&ent(5.0)));
    }

    #[test]
    fn rejects_when_value_below() {
        let p = NumericGte::new("arrondissement".into(), 5.0, labels());
        assert!(!p.matches(&ent(4.0)));
        assert!(!p.matches(&ent(1.0)));
    }
}
