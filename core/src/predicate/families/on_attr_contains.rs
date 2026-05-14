//! `on_attr_contains(attr, value)` — true when the entity declares attribute `attr`
//! of type `str_list` AND that list contains `value`. Mirrors `on_attr_in_set` for
//! the single-needle case.

use crate::{
    entity::{AttributeValue, Entity},
    error::Result,
    predicate::{
        families::common::{entity_attr, invalid_param, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::sync::Arc;

const FAMILY: &str = "on_attr_contains";

#[derive(Debug)]
pub struct OnAttrContains {
    id: String,
    attr: String,
    needle: String,
    labels: StoredLabels,
}

impl OnAttrContains {
    #[must_use]
    pub fn new(attr: String, needle: String, labels: StoredLabels) -> Self {
        let id = predicate_id(FAMILY, &format!("{attr}:{needle}"));
        Self {
            id,
            attr,
            needle,
            labels,
        }
    }
}

impl Predicate for OnAttrContains {
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
        match entity_attr(entity, &self.attr) {
            Some(AttributeValue::StrList(list)) => list.iter().any(|s| s == &self.needle),
            Some(AttributeValue::Str(s)) => s == &self.needle,
            _ => false,
        }
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
        let needle = obj
            .get("value")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_param(FAMILY, "missing string 'value'"))?
            .to_string();
        Ok(Arc::new(OnAttrContains::new(
            attr,
            needle,
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
                text: "Contient".into(),
                help: None,
            },
            en: None,
            other: BTreeMap::new(),
        })
    }

    fn ent(attrs: HashMap<String, AttributeValue>) -> Entity {
        Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: attrs,
            fame_score: None,
            icon_url: None,
        }
    }

    #[test]
    fn matches_when_list_contains_value() {
        let mut a = HashMap::new();
        a.insert(
            "lines".into(),
            AttributeValue::StrList(vec!["1".into(), "4".into()]),
        );
        let p = OnAttrContains::new("lines".into(), "1".into(), labels());
        assert!(p.matches(&ent(a)));
    }

    #[test]
    fn rejects_when_list_misses_value() {
        let mut a = HashMap::new();
        a.insert("lines".into(), AttributeValue::StrList(vec!["3".into()]));
        let p = OnAttrContains::new("lines".into(), "1".into(), labels());
        assert!(!p.matches(&ent(a)));
    }
}
