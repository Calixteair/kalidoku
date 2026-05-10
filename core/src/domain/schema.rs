//! JSON Schema validation for domain pack manifests.
//!
//! The contract lives in `contracts/entity-schema.json` and is embedded at compile
//! time via `include_str!`. The shared `$defs` section is reused to validate each of
//! the three manifest files separately:
//!
//! | manifest          | validated against `$defs` |
//! |-------------------|---------------------------|
//! | `metadata.json`   | `DomainMetadata`          |
//! | `entities.json`   | array of `Entity`         |
//! | `predicates.json` | array of `PredicateDefinition` |
//!
//! Validation runs **before** serde deserialization in [`crate::domain::loader`], so
//! shape errors surface as a typed [`crate::error::Error::InvalidManifest`] with a
//! list of human-readable messages instead of a generic serde error.
//!
//! The compiled validators are cached in a `OnceLock` — compiling JSON Schema is
//! the expensive part, and the engine uses the same three validators forever.

use std::sync::OnceLock;

use jsonschema::JSONSchema;
use serde_json::{json, Value};

use crate::error::{Error, Result};

/// Raw schema source pulled in from the workspace contract.
const ENTITY_SCHEMA_SRC: &str = include_str!("../../../contracts/entity-schema.json");

/// Which kind of manifest we are validating. Each variant maps to one `$defs`
/// definition inside `contracts/entity-schema.json`.
#[derive(Debug, Clone, Copy)]
pub enum ManifestKind {
    Metadata,
    Entities,
    Predicates,
}

impl ManifestKind {
    fn def_name(self) -> &'static str {
        match self {
            Self::Metadata => "DomainMetadata",
            Self::Entities => "Entity",
            Self::Predicates => "PredicateDefinition",
        }
    }

    fn is_array(self) -> bool {
        matches!(self, Self::Entities | Self::Predicates)
    }
}

/// Lazily-compiled, cached validator triplet (one per [`ManifestKind`]).
struct CompiledValidators {
    metadata: JSONSchema,
    entities: JSONSchema,
    predicates: JSONSchema,
}

fn validators() -> &'static CompiledValidators {
    static CELL: OnceLock<CompiledValidators> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw: Value = serde_json::from_str(ENTITY_SCHEMA_SRC)
            .expect("contracts/entity-schema.json must be valid JSON");
        CompiledValidators {
            metadata: compile_for(&raw, ManifestKind::Metadata),
            entities: compile_for(&raw, ManifestKind::Entities),
            predicates: compile_for(&raw, ManifestKind::Predicates),
        }
    })
}

/// Build a wrapper schema that points at `$defs/<def_name>` (optionally wrapped in
/// an array) and reuses the original `$defs` for `$ref` resolution.
fn compile_for(raw: &Value, kind: ManifestKind) -> JSONSchema {
    let defs = raw
        .get("$defs")
        .cloned()
        .expect("contracts/entity-schema.json must declare $defs");
    let item_ref = json!({ "$ref": format!("#/$defs/{}", kind.def_name()) });
    let mut wrapper = if kind.is_array() {
        json!({
            "type": "array",
            "items": item_ref,
        })
    } else {
        item_ref
    };
    wrapper
        .as_object_mut()
        .expect("wrapper schema is always an object")
        .insert("$defs".to_string(), defs);
    JSONSchema::compile(&wrapper).expect("contracts/entity-schema.json must compile")
}

/// Validate a parsed JSON instance against the embedded contract.
///
/// On failure returns [`Error::InvalidManifest`] containing every reported error,
/// prefixed by the JSON pointer to the offending location. The returned vector is
/// capped at 32 entries to keep error messages bounded.
pub fn validate(path: &str, kind: ManifestKind, instance: &Value) -> Result<()> {
    let validator = match kind {
        ManifestKind::Metadata => &validators().metadata,
        ManifestKind::Entities => &validators().entities,
        ManifestKind::Predicates => &validators().predicates,
    };
    if validator.is_valid(instance) {
        return Ok(());
    }
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .take(32)
        .map(|e| format!("at {}: {}", e.instance_path, e))
        .collect();
    Err(Error::InvalidManifest {
        path: path.to_string(),
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_entity_schema_from_contracts() {
        let _v = validators();
    }

    #[test]
    fn metadata_minimal_is_valid() {
        let m = json!({
            "id": "paris-metro",
            "name": { "fr": "Métro de Paris" },
            "version": "1.0.0",
            "default_locale": "fr"
        });
        validate("memory:metadata", ManifestKind::Metadata, &m).expect("valid metadata");
    }

    #[test]
    fn metadata_missing_required_field_is_rejected() {
        let m = json!({
            "id": "paris-metro",
            "version": "1.0.0",
            "default_locale": "fr"
        });
        let err = validate("memory:metadata", ManifestKind::Metadata, &m).unwrap_err();
        match err {
            Error::InvalidManifest { path, errors } => {
                assert_eq!(path, "memory:metadata");
                assert!(!errors.is_empty(), "expected at least one error");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn entities_array_is_valid() {
        let e = json!([
            {
                "id": "abbesses",
                "name": "Abbesses",
                "attributes": {
                    "lines": { "str_list": ["12"] },
                    "geo": { "geo": { "lat": 48.88, "lon": 2.34 } }
                }
            }
        ]);
        validate("memory:entities", ManifestKind::Entities, &e).expect("valid entities");
    }

    #[test]
    fn entities_with_bad_id_is_rejected() {
        let e = json!([
            { "id": "Abbesses!", "name": "Abbesses", "attributes": {} }
        ]);
        let err = validate("memory:entities", ManifestKind::Entities, &e).unwrap_err();
        assert!(matches!(err, Error::InvalidManifest { .. }));
    }

    #[test]
    fn predicates_with_unknown_family_is_rejected() {
        let p = json!([
            {
                "family": "totally_unknown_family",
                "labels": { "fr": { "text": "x" } }
            }
        ]);
        let err = validate("memory:predicates", ManifestKind::Predicates, &p).unwrap_err();
        assert!(matches!(err, Error::InvalidManifest { .. }));
    }

    #[test]
    fn predicates_with_known_family_are_accepted() {
        let p = json!([
            {
                "family": "ends_with",
                "param": "S",
                "labels": { "fr": { "text": "Termine par S" } }
            },
            {
                "family": "attr_list_size_gte",
                "param": { "attr": "lines", "n": 3 },
                "labels": { "fr": { "text": "Au moins 3 lignes" } }
            }
        ]);
        validate("memory:predicates", ManifestKind::Predicates, &p).expect("valid predicates");
    }
}
