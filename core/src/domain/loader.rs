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
    domain::{
        schema::{validate as validate_against_schema, ManifestKind},
        Domain, DomainMetadata,
    },
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
/// - invalid JSON syntax → `Error::Json`
/// - shape mismatch with `contracts/entity-schema.json` → `Error::InvalidManifest`
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
    let metadata: DomainMetadata =
        read_validated_json(&dir.join(FILE_METADATA), ManifestKind::Metadata)?;
    let entities: Vec<Entity> =
        read_validated_json(&dir.join(FILE_ENTITIES), ManifestKind::Entities)?;
    let predicate_defs: Vec<PredicateDefinition> =
        read_validated_json(&dir.join(FILE_PREDICATES), ManifestKind::Predicates)?;
    Domain::load(metadata, entities, &predicate_defs, registry)
}

/// Read a manifest, parse it once into [`serde_json::Value`], validate the shape
/// against the contract, then reify into the strongly-typed `T`. The intermediate
/// `Value` is cheap and lets the JSON-Schema layer surface precise errors before
/// the domain-specific deserialization runs.
fn read_validated_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    kind: ManifestKind,
) -> Result<T> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    validate_against_schema(&path.display().to_string(), kind, &value)?;
    let parsed = serde_json::from_value(value)?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    fn write_pack(metadata: &str, entities: &str, predicates: &str) -> tempdir_polyfill::TempDir {
        let dir =
            tempdir_polyfill::TempDir::new("kalidoku-loader-test").expect("create temp dir");
        fs::write(dir.path().join(FILE_METADATA), metadata).expect("write metadata");
        fs::write(dir.path().join(FILE_ENTITIES), entities).expect("write entities");
        fs::write(dir.path().join(FILE_PREDICATES), predicates).expect("write predicates");
        dir
    }

    const VALID_METADATA: &str = r#"{
        "id": "test-domain",
        "name": { "fr": "Test" },
        "version": "1.0.0",
        "default_locale": "fr"
    }"#;

    const VALID_ENTITIES: &str = r#"[
        {
            "id": "alpha",
            "name": "Alpha",
            "attributes": { "lines": { "str_list": ["1"] } }
        },
        {
            "id": "beta",
            "name": "Beta",
            "attributes": { "lines": { "str_list": ["2"] } }
        }
    ]"#;

    const VALID_PREDICATES: &str = r#"[
        {
            "family": "starts_with",
            "param": "A",
            "labels": { "fr": { "text": "Commence par A" } }
        }
    ]"#;

    #[test]
    fn valid_manifest_loads_successfully() {
        let pack = write_pack(VALID_METADATA, VALID_ENTITIES, VALID_PREDICATES);
        let domain = load_domain_pack(pack.path()).expect("valid pack should load");
        assert_eq!(domain.metadata.id, "test-domain");
        assert_eq!(domain.entities.len(), 2);
        assert_eq!(domain.predicates.len(), 1);
    }

    #[test]
    fn metadata_with_bad_id_returns_invalid_manifest() {
        // Uppercase id violates the `[a-z0-9][a-z0-9-]{0,63}` pattern.
        let bad_metadata = r#"{
            "id": "Test-Domain",
            "name": { "fr": "Test" },
            "version": "1.0.0",
            "default_locale": "fr"
        }"#;
        let pack = write_pack(bad_metadata, VALID_ENTITIES, VALID_PREDICATES);
        let err = load_domain_pack(pack.path()).expect_err("should reject bad metadata");
        match err {
            Error::InvalidManifest { path, errors } => {
                assert!(
                    path.contains(FILE_METADATA),
                    "path should point at metadata: {path}"
                );
                assert!(!errors.is_empty(), "expected at least one schema error");
            }
            other => panic!("expected InvalidManifest, got {other:?}"),
        }
    }

    #[test]
    fn predicates_with_unknown_family_returns_invalid_manifest() {
        let bad_predicates = r#"[
            {
                "family": "totally_unknown",
                "param": "X",
                "labels": { "fr": { "text": "x" } }
            }
        ]"#;
        let pack = write_pack(VALID_METADATA, VALID_ENTITIES, bad_predicates);
        let err = load_domain_pack(pack.path()).expect_err("should reject unknown family");
        assert!(matches!(err, Error::InvalidManifest { .. }));
    }
}

/// Tiny `tempdir`-like helper kept inline so `core/` does not pull a new dev-dep
/// just for this single test module. Cleans the directory on drop.
#[cfg(test)]
mod tempdir_polyfill {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn new(prefix: &str) -> std::io::Result<Self> {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let pid = std::process::id();
            let path = std::env::temp_dir().join(format!("{prefix}-{pid}-{n}"));
            fs::create_dir_all(&path)?;
            Ok(Self { path })
        }

        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
