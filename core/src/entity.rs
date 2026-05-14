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
    /// Intrinsic notoriety on 0..=100. Higher = more famous. None means
    /// the dataset hasn't been scored yet — engine treats it as 50 (neutral)
    /// when computing the originality score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fame_score: Option<u8>,
    /// Optional URL to a representative image. The frontend downloads + caches
    /// these at ingest time into `web/public/cards/<domain>/<id>.png` and
    /// renders them via CardIcon.svelte. Engine logic does nothing with this
    /// field, it's pure presentation metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

pub const FAME_NEUTRAL: u8 = 50;
pub const FAME_MAX_PER_CELL: u32 = 100;

impl Entity {
    #[must_use]
    pub fn attr(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    /// Returns the fame_score clamped to [0, 100], or 50 if absent.
    #[must_use]
    pub fn effective_fame(&self) -> u8 {
        self.fame_score.map_or(FAME_NEUTRAL, |f| f.min(100))
    }
}
