//! On-disk layout for a domain pack:
//!
//! ```text
//! domains/<id>/
//!   ├── metadata.json
//!   ├── predicates.json
//!   └── entities.json
//! ```
//!
//! `core/` only owns this synchronous JSON loader. Anything fancier (versioning,
//! signing, S3 caching) belongs in `worker/` or `server/`.

use std::path::Path;

use crate::{
    domain::{Domain, DomainMetadata},
    entity::Entity,
    error::{Error, Result},
    predicate::{PredicateDefinition, PredicateRegistry},
};

const FILE_METADATA: &str = "metadata.json";
const FILE_PREDICATES: &str = "predicates.json";
const FILE_ENTITIES: &str = "entities.json";

/// Read every file of a domain pack and build a [`Domain`] using the default
/// [`PredicateRegistry`]. Failure modes:
///
/// - missing or unreadable file → `Error::Io`
/// - invalid JSON → `Error::Json`
/// - unknown predicate family → `Error::UnknownPredicateFamily`
/// - empty entity list → `Error::InvalidDomain`
pub fn load_domain_pack(dir: &Path) -> Result<Domain> {
    load_domain_pack_with_registry(dir, &PredicateRegistry::with_defaults())
}

/// Same as [`load_domain_pack`] but uses an explicit registry — useful for tests
/// that register a custom predicate family without polluting the default set.
pub fn load_domain_pack_with_registry(dir: &Path, registry: &PredicateRegistry) -> Result<Domain> {
    if !dir.is_dir() {
        return Err(Error::InvalidDomain(format!(
            "{} is not a directory",
            dir.display()
        )));
    }
    let metadata: DomainMetadata = read_json(&dir.join(FILE_METADATA))?;
    let entities: Vec<Entity> = read_json(&dir.join(FILE_ENTITIES))?;
    let predicate_defs: Vec<PredicateDefinition> = read_json(&dir.join(FILE_PREDICATES))?;
    Domain::load(metadata, entities, &predicate_defs, registry)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let parsed = serde_json::from_slice(&bytes)?;
    Ok(parsed)
}
