use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeValue {
    Str(String),
    Num(f64),
    Bool(bool),
    StrList(Vec<String>),
    Geo(GeoPoint),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub attributes: HashMap<String, AttributeValue>,
}

impl Entity {
    #[must_use]
    pub fn attr(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }
}
