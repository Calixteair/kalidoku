//! `on_attr_in_set(attr, values)` — true when the entity declares attribute `attr`
//! and at least one of its values is in `values`. Works on both scalar string
//! attributes and `str_list` attributes.

use crate::{
    entity::{AttributeValue, Entity},
    error::Result,
    predicate::{
        families::common::{entity_attr, invalid_param, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use std::{collections::HashSet, sync::Arc};

const FAMILY: &str = "on_attr_in_set";

#[derive(Debug)]
pub struct OnAttrInSet {
    id: String,
    attr: String,
    values: HashSet<String>,
    labels: StoredLabels,
}

impl OnAttrInSet {
    #[must_use]
    pub fn new(attr: String, values: Vec<String>, labels: StoredLabels) -> Self {
        let mut sorted = values.clone();
        sorted.sort();
        let id = predicate_id(FAMILY, &format!("{}:{}", attr, sorted.join(",")));
        Self {
            id,
            attr,
            values: values.into_iter().collect(),
            labels,
        }
    }
}

impl Predicate for OnAttrInSet {
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
            Some(AttributeValue::Str(s)) => self.values.contains(s),
            Some(AttributeValue::StrList(list)) => list.iter().any(|s| self.values.contains(s)),
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
            .ok_or_else(|| invalid_param(FAMILY, "expected object {attr, values}"))?;
        let attr = obj
            .get("attr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_param(FAMILY, "missing string 'attr'"))?
            .to_string();
        let values_arr = obj
            .get("values")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| invalid_param(FAMILY, "missing array 'values'"))?;
        let values = values_arr
            .iter()
            .map(|v| {
                v.as_str()
                    .map(String::from)
                    .ok_or_else(|| invalid_param(FAMILY, "values must all be strings"))
            })
            .collect::<Result<Vec<_>>>()?;
        if values.is_empty() {
            return Err(invalid_param(
                FAMILY,
                "values must contain at least one item",
            ));
        }
        Ok(Arc::new(OnAttrInSet::new(attr, values, stored_labels(def))))
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
                text: "Set".into(),
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
            fame_score: None,
            icon_url: None,
        }
    }

    #[test]
    fn matches_when_str_list_intersects() {
        let mut a = HashMap::new();
        a.insert(
            "lines".into(),
            AttributeValue::StrList(vec!["1".into(), "4".into(), "7".into()]),
        );
        let p = OnAttrInSet::new("lines".into(), vec!["1".into(), "14".into()], labels());
        assert!(p.matches(&ent_with_attrs(a)));
    }

    #[test]
    fn matches_when_str_attr_in_set() {
        let mut a = HashMap::new();
        a.insert("zone".into(), AttributeValue::Str("center".into()));
        let p = OnAttrInSet::new(
            "zone".into(),
            vec!["edge".into(), "center".into()],
            labels(),
        );
        assert!(p.matches(&ent_with_attrs(a)));
    }

    #[test]
    fn rejects_disjoint_lists() {
        let mut a = HashMap::new();
        a.insert("lines".into(), AttributeValue::StrList(vec!["3".into()]));
        let p = OnAttrInSet::new("lines".into(), vec!["1".into(), "4".into()], labels());
        assert!(!p.matches(&ent_with_attrs(a)));
    }
}
