use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    entity::Entity,
    error::{Error, Result},
};

pub mod families;

/// A predicate decides whether an entity satisfies a clue.
/// Implementations live in `core::predicate::families::*` and are registered at domain load.
pub trait Predicate: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn family(&self) -> &str;
    fn label(&self, locale: &str) -> &str;
    fn help(&self, locale: &str) -> Option<&str>;
    fn matches(&self, entity: &Entity) -> bool;
}

pub type DynPredicate = Arc<dyn Predicate>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateDefinition {
    pub family: String,
    #[serde(default)]
    pub param: serde_json::Value,
    pub labels: PredicateLabels,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateLabels {
    pub fr: Label,
    #[serde(default)]
    pub en: Option<Label>,
    #[serde(flatten)]
    pub other: std::collections::BTreeMap<String, Label>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub text: String,
    #[serde(default)]
    pub help: Option<String>,
}

impl PredicateLabels {
    /// Returns the label for the requested locale, falling back to `fr` then `en`.
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

/// Build a `Predicate` from a serialised definition. Each predicate family registers a builder.
pub trait PredicateFactory: Send + Sync {
    fn family(&self) -> &'static str;
    fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate>;
}

pub struct PredicateRegistry {
    factories: Vec<Box<dyn PredicateFactory>>,
}

impl PredicateRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    /// Registry pre-populated with every built-in family declared by `core/`.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut r = Self::new();
        families::register_defaults(&mut r);
        r
    }

    pub fn register<F: PredicateFactory + 'static>(&mut self, factory: F) {
        self.factories.push(Box::new(factory));
    }

    pub fn build(&self, def: &PredicateDefinition) -> Result<DynPredicate> {
        self.factories
            .iter()
            .find(|f| f.family() == def.family)
            .ok_or_else(|| Error::UnknownPredicateFamily(def.family.clone()))?
            .build(def)
    }

    /// Read-only access to declared families (for diagnostics / tests).
    #[must_use]
    pub fn families(&self) -> Vec<&'static str> {
        self.factories.iter().map(|f| f.family()).collect()
    }
}

impl Default for PredicateRegistry {
    fn default() -> Self {
        Self::new()
    }
}
