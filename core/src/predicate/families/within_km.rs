//! `within_km(attr, point, km)` — true when the entity declares a `geo` attribute
//! whose haversine distance from the anchor point is at most `km` kilometres.
//! Distance computation uses [`geoutils::Location::haversine_distance_to`].

use crate::{
    entity::{AttributeValue, Entity, GeoPoint},
    error::Result,
    predicate::{
        families::common::{entity_attr, invalid_param, predicate_id, stored_labels, StoredLabels},
        DynPredicate, Predicate, PredicateDefinition, PredicateFactory,
    },
};
use geoutils::Location;
use std::sync::Arc;

const FAMILY: &str = "within_km";

#[derive(Debug)]
pub struct WithinKm {
    id: String,
    attr: String,
    anchor: GeoPoint,
    radius_m: f64,
    labels: StoredLabels,
}

impl WithinKm {
    #[must_use]
    pub fn new(attr: String, anchor: GeoPoint, km: f64, labels: StoredLabels) -> Self {
        let key = format!("{}:{:.4}:{:.4}:{:.2}", attr, anchor.lat, anchor.lon, km);
        Self {
            id: predicate_id(FAMILY, &key),
            attr,
            anchor,
            radius_m: km * 1000.0,
            labels,
        }
    }
}

impl Predicate for WithinKm {
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
        let Some(AttributeValue::Geo(p)) = entity_attr(entity, &self.attr) else {
            return false;
        };
        let from = Location::new(self.anchor.lat, self.anchor.lon);
        let to = Location::new(p.lat, p.lon);
        match from.distance_to(&to) {
            Ok(d) => d.meters() <= self.radius_m,
            Err(_) => false,
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
            .ok_or_else(|| invalid_param(FAMILY, "expected object {attr, from:{lat,lon}, km}"))?;
        let attr = obj
            .get("attr")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_param(FAMILY, "missing string 'attr'"))?
            .to_string();
        let from = obj
            .get("from")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| invalid_param(FAMILY, "missing object 'from'"))?;
        let lat = from
            .get("lat")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| invalid_param(FAMILY, "missing number 'from.lat'"))?;
        let lon = from
            .get("lon")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| invalid_param(FAMILY, "missing number 'from.lon'"))?;
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(invalid_param(FAMILY, "lat/lon out of range"));
        }
        let km = obj
            .get("km")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| invalid_param(FAMILY, "missing number 'km'"))?;
        if !km.is_finite() || km <= 0.0 {
            return Err(invalid_param(FAMILY, "'km' must be a positive number"));
        }
        Ok(Arc::new(WithinKm::new(
            attr,
            GeoPoint { lat, lon },
            km,
            stored_labels(def),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::{AttributeValue, GeoPoint},
        predicate::{Label, PredicateLabels},
    };
    use std::collections::{BTreeMap, HashMap};

    fn labels() -> StoredLabels {
        StoredLabels::from(&PredicateLabels {
            fr: Label {
                text: "Proche".into(),
                help: None,
            },
            en: None,
            other: BTreeMap::new(),
        })
    }

    fn ent(lat: f64, lon: f64) -> Entity {
        let mut a = HashMap::new();
        a.insert("geo".into(), AttributeValue::Geo(GeoPoint { lat, lon }));
        Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: a,
        }
    }

    #[test]
    fn matches_when_close_enough() {
        // Notre-Dame anchor; Saint-Michel is ~50m away.
        let p = WithinKm::new(
            "geo".into(),
            GeoPoint {
                lat: 48.852968,
                lon: 2.349902,
            },
            3.0,
            labels(),
        );
        assert!(p.matches(&ent(48.853424, 2.344166)));
    }

    #[test]
    fn rejects_when_too_far() {
        let p = WithinKm::new(
            "geo".into(),
            GeoPoint {
                lat: 48.852968,
                lon: 2.349902,
            },
            1.0,
            labels(),
        );
        // Étoile is ~4km away from Notre-Dame.
        assert!(!p.matches(&ent(48.873779, 2.295269)));
    }

    #[test]
    fn rejects_when_attribute_missing() {
        let p = WithinKm::new(
            "geo".into(),
            GeoPoint { lat: 0.0, lon: 0.0 },
            10.0,
            labels(),
        );
        let e = Entity {
            id: "x".into(),
            name: "X".into(),
            aliases: vec![],
            attributes: HashMap::new(),
        };
        assert!(!p.matches(&e));
    }
}
