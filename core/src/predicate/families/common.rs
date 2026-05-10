//! Shared helpers for predicate families.

use crate::{
    entity::AttributeValue,
    error::{Error, Result},
    normalize::normalize,
    predicate::{Label, PredicateDefinition, PredicateLabels},
};

/// Build a stable, deterministic predicate id from a family name + a textual key.
/// Family ids are used for tie-breaking in the generator and as anchors in the public Grid.
#[must_use]
pub fn predicate_id(family: &str, key: &str) -> String {
    let mut out = String::with_capacity(family.len() + key.len() + 1);
    out.push_str(family);
    out.push(':');
    for c in key.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == '-' || c == '_' || c == '.' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}

/// Reject a predicate that has the wrong shape with a useful error message.
pub fn invalid_param(family: &str, reason: impl Into<String>) -> Error {
    Error::InvalidPredicateParam {
        family: family.into(),
        reason: reason.into(),
    }
}

/// Extract a borrowed string from a serde value, returning a typed error otherwise.
pub fn param_as_str<'a>(family: &str, value: &'a serde_json::Value) -> Result<&'a str> {
    value
        .as_str()
        .ok_or_else(|| invalid_param(family, "expected a string parameter"))
}

/// Extract an unsigned integer parameter (rejects negatives / non-integers).
pub fn param_as_u32(family: &str, value: &serde_json::Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| invalid_param(family, "expected a non-negative integer"))
}

/// Extract a finite f64 parameter.
pub fn param_as_f64(family: &str, value: &serde_json::Value) -> Result<f64> {
    value
        .as_f64()
        .ok_or_else(|| invalid_param(family, "expected a number"))
}

/// Convenience: read a single character (case-insensitive) from a string parameter.
pub fn param_as_letter(family: &str, value: &serde_json::Value) -> Result<char> {
    let s = param_as_str(family, value)?;
    let mut chars = s.chars();
    let first = chars
        .next()
        .ok_or_else(|| invalid_param(family, "expected a non-empty string"))?;
    if chars.next().is_some() {
        return Err(invalid_param(
            family,
            "expected exactly one character (letter or digit)",
        ));
    }
    Ok(first)
}

/// Normalised lowercase representation of an entity name (no accents).
#[must_use]
pub fn normalised_name(name: &str) -> String {
    normalize(name)
}

/// Normalised representation of a free letter parameter (used by `ends_with`,
/// `starts_with`, `contains_letter`).
#[must_use]
pub fn normalised_letter(c: char) -> String {
    normalize(&c.to_string())
}

/// Resolve an attribute by key. Returns `None` if the entity does not declare this attribute.
#[must_use]
pub fn entity_attr<'a>(entity: &'a crate::entity::Entity, key: &str) -> Option<&'a AttributeValue> {
    entity.attributes.get(key)
}

/// Carrier for the (cloned) localisation labels every concrete predicate stores.
#[derive(Debug, Clone)]
pub struct StoredLabels {
    pub fr: Label,
    pub en: Option<Label>,
    pub other: std::collections::BTreeMap<String, Label>,
}

impl StoredLabels {
    #[must_use]
    pub fn from(def: &PredicateLabels) -> Self {
        Self {
            fr: def.fr.clone(),
            en: def.en.clone(),
            other: def.other.clone(),
        }
    }

    /// Pick a label with locale fallback (locale → fr → en → first other).
    #[must_use]
    pub fn pick(&self, locale: &str) -> &Label {
        if locale == "fr" {
            return &self.fr;
        }
        if locale == "en" {
            if let Some(en) = self.en.as_ref() {
                return en;
            }
        }
        if let Some(other) = self.other.get(locale) {
            return other;
        }
        if let Some(en) = self.en.as_ref() {
            return en;
        }
        &self.fr
    }
}

/// Helper to build the labels carrier from a definition.
#[must_use]
pub fn stored_labels(def: &PredicateDefinition) -> StoredLabels {
    StoredLabels::from(&def.labels)
}
