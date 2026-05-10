use serde::{Deserialize, Serialize};

use crate::{
    entity::Entity,
    error::{Error, Result},
    predicate::{DynPredicate, PredicateDefinition, PredicateRegistry},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMetadata {
    pub id: String,
    pub name: std::collections::BTreeMap<String, String>,
    pub version: String,
    pub default_locale: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub generator_constraints: GeneratorConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneratorConstraints {
    pub min_candidates_per_cell: u32,
    pub max_candidates_per_cell: u32,
    pub max_predicate_family_repeats: u32,
    pub row_predicate_count: u32,
    pub col_predicate_count: u32,
}

impl Default for GeneratorConstraints {
    fn default() -> Self {
        Self {
            min_candidates_per_cell: 2,
            max_candidates_per_cell: 30,
            max_predicate_family_repeats: 1,
            row_predicate_count: 3,
            col_predicate_count: 3,
        }
    }
}

#[derive(Debug)]
pub struct Domain {
    pub metadata: DomainMetadata,
    pub entities: Vec<Entity>,
    pub predicates: Vec<DynPredicate>,
}

impl Domain {
    pub fn load(
        metadata: DomainMetadata,
        entities: Vec<Entity>,
        predicate_defs: &[PredicateDefinition],
        registry: &PredicateRegistry,
    ) -> Result<Self> {
        if entities.is_empty() {
            return Err(Error::InvalidDomain(format!(
                "domain '{}' has no entities",
                metadata.id
            )));
        }
        let predicates = predicate_defs
            .iter()
            .map(|d| registry.build(d))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            metadata,
            entities,
            predicates,
        })
    }
}
