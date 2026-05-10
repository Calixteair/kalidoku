//! Built-in predicate families. Each family is registered by
//! [`crate::predicate::PredicateRegistry::with_defaults`].
//!
//! Adding a new family means: create a sibling file, expose a `Factory` struct
//! implementing [`crate::predicate::PredicateFactory`], then register it in
//! [`register_defaults`].

mod attr_list_size_eq;
mod attr_list_size_gte;
mod common;
mod contains_letter;
mod ends_with;
mod name_length_eq;
mod name_length_max;
mod name_length_min;
mod numeric_between;
mod numeric_gte;
mod numeric_lte;
mod on_attr_contains;
mod on_attr_eq;
mod on_attr_in_set;
mod starts_with;
mod within_km;

pub use common::predicate_id;

use super::PredicateRegistry;

/// Register every built-in family on the given registry.
pub fn register_defaults(registry: &mut PredicateRegistry) {
    registry.register(ends_with::Factory);
    registry.register(starts_with::Factory);
    registry.register(contains_letter::Factory);
    registry.register(name_length_max::Factory);
    registry.register(name_length_min::Factory);
    registry.register(name_length_eq::Factory);
    registry.register(on_attr_eq::Factory);
    registry.register(on_attr_in_set::Factory);
    registry.register(on_attr_contains::Factory);
    registry.register(within_km::Factory);
    registry.register(numeric_gte::Factory);
    registry.register(numeric_lte::Factory);
    registry.register(numeric_between::Factory);
    registry.register(attr_list_size_gte::Factory);
    registry.register(attr_list_size_eq::Factory);
}
