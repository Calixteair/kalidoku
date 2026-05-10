use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid domain pack: {0}")]
    InvalidDomain(String),

    #[error("unknown predicate family: {0}")]
    UnknownPredicateFamily(String),

    #[error("invalid predicate parameter for family {family}: {reason}")]
    InvalidPredicateParam { family: String, reason: String },

    #[error("entity {id}: missing required attribute '{attr}'")]
    MissingAttribute { id: String, attr: String },

    #[error("generator failed to find a valid grid after {attempts} attempts")]
    GeneratorExhausted { attempts: u32 },

    #[error("attribute type mismatch on {entity_id}.{attr}: expected {expected}, got {actual}")]
    AttributeTypeMismatch {
        entity_id: String,
        attr: String,
        expected: &'static str,
        actual: &'static str,
    },

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
