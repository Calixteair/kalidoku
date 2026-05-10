//! `numeric_between(attr, min, max)` — true when the entity declares numeric
//! attribute `attr` with a value in `[min, max]` (bounds inclusive).

use crate::{
    entity::{AttributeValue, Entity},
    error::Result,
    predicate::{
        families::common::{entity_attr, invalid_param, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "numeric_between";

#[derive(Debug)]
pub struct NumericBetween {
    id: String,
    attr: String,
    min: f64,
    max: f64,
    labels: StoredLabels,
}

impl NumericBetween {
    #[must_use]
    pub fn new(attr: String, min: f64, max: f64, labels: StoredLabels) -> Self {
        Self {
            id: predicate_id(FAMILY, &format!("{attr}:{min}-{max}")),
            attr,
            min,
            max,
            labels,
        }
    }
}

impl Predicate for NumericBetween {
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
            Some(AttributeValue::Num(n)) if *n >= self.min && *n <= self.max
        )
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
            .ok_or_else(|| invalid_param(FAMILY, "expected object {attr, min, max}"))?;
        let attr = obj
            .get("attr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_param(FAMILY, "missing string 'attr'"))?
            .to_string();
        let min = obj
            .get("min")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| invalid_param(FAMILY, "missing number 'min'"))?;
        let max = obj
            .get("max")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| invalid_param(FAMILY, "missing number 'max'"))?;
        if min > max {
            return Err(invalid_param(FAMILY, "min must be <= max"));
        }
        Ok(Arc::new(NumericBetween::new(
            attr,
            min,
            max,
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
                text: "Entre".into(),
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
        }
    }

    #[test]
    fn matches_when_in_range_inclusive() {
        let p = NumericBetween::new("arrondissement".into(), 1.0, 8.0, labels());
        assert!(p.matches(&ent(1.0)));
        assert!(p.matches(&ent(5.0)));
        assert!(p.matches(&ent(8.0)));
    }

    #[test]
    fn rejects_when_outside_range() {
        let p = NumericBetween::new("arrondissement".into(), 1.0, 8.0, labels());
        assert!(!p.matches(&ent(0.0)));
        assert!(!p.matches(&ent(9.0)));
    }
}
