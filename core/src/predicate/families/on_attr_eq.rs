//! `on_attr_eq(attr, value)` — true when the entity declares attribute `attr` and
//! its value equals `value`. Supports `str`, `num`, `bool`. For `str_list` / `geo`
//! the predicate is rejected at build time (use `on_attr_contains` / `within_km`).

use crate::{
    entity::{AttributeValue, Entity},
    error::Result,
    predicate::{
        families::common::{entity_attr, invalid_param, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "on_attr_eq";

#[derive(Debug, Clone)]
pub enum Scalar {
    Str(String),
    Num(f64),
    Bool(bool),
}

impl Scalar {
    fn matches_attr(&self, value: &AttributeValue) -> bool {
        match (self, value) {
            (Scalar::Str(a), AttributeValue::Str(b)) => a == b,
            (Scalar::Num(a), AttributeValue::Num(b)) => (a - b).abs() < f64::EPSILON,
            (Scalar::Bool(a), AttributeValue::Bool(b)) => a == b,
            _ => false,
        }
    }

    fn key(&self) -> String {
        match self {
            Scalar::Str(s) => s.clone(),
            Scalar::Num(n) => n.to_string(),
            Scalar::Bool(b) => b.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct OnAttrEq {
    id: String,
    attr: String,
    value: Scalar,
    labels: StoredLabels,
}

impl OnAttrEq {
    #[must_use]
    pub fn new(attr: String, value: Scalar, labels: StoredLabels) -> Self {
        let id = predicate_id(FAMILY, &format!("{}={}", attr, value.key()));
        Self {
            id,
            attr,
            value,
            labels,
        }
    }
}

impl Predicate for OnAttrEq {
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
        entity_attr(entity, &self.attr).is_some_and(|v| self.value.matches_attr(v))
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
            .ok_or_else(|| invalid_param(FAMILY, "expected object {attr, value}"))?;
        let attr = obj
            .get("attr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_param(FAMILY, "missing string 'attr'"))?
            .to_string();
        let raw_value = obj
            .get("value")
            .ok_or_else(|| invalid_param(FAMILY, "missing 'value'"))?;
        let value = if let Some(s) = raw_value.as_str() {
            Scalar::Str(s.to_string())
        } else if let Some(b) = raw_value.as_bool() {
            Scalar::Bool(b)
        } else if let Some(n) = raw_value.as_f64() {
            Scalar::Num(n)
        } else {
            return Err(invalid_param(
                FAMILY,
                "value must be a string, number or boolean",
            ));
        };
        Ok(Arc::new(OnAttrEq::new(attr, value, stored_labels(def))))
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
                text: "Égal".into(),
                help: None,
            },
            en: None,
            other: BTreeMap::new(),
        })
    }

    fn ent_with_attrs(attrs: HashMap<String, AttributeValue>) -> Entity {
        Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: attrs,
        }
    }

    #[test]
    fn matches_string_equality() {
        let mut a = HashMap::new();
        a.insert("zone".into(), AttributeValue::Str("center".into()));
        let p = OnAttrEq::new("zone".into(), Scalar::Str("center".into()), labels());
        assert!(p.matches(&ent_with_attrs(a)));
    }

    #[test]
    fn rejects_when_attribute_missing() {
        let p = OnAttrEq::new("zone".into(), Scalar::Str("center".into()), labels());
        assert!(!p.matches(&ent_with_attrs(HashMap::new())));
    }

    #[test]
    fn matches_numeric_equality() {
        let mut a = HashMap::new();
        a.insert("arrondissement".into(), AttributeValue::Num(5.0));
        let p = OnAttrEq::new("arrondissement".into(), Scalar::Num(5.0), labels());
        assert!(p.matches(&ent_with_attrs(a)));
    }

    #[test]
    fn matches_boolean_equality() {
        let mut a = HashMap::new();
        a.insert("in_paris".into(), AttributeValue::Bool(true));
        let p = OnAttrEq::new("in_paris".into(), Scalar::Bool(true), labels());
        assert!(p.matches(&ent_with_attrs(a)));
    }
}
