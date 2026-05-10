#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unreadable_literal,
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::too_many_lines,
    dead_code
)]

pub mod domain;
pub mod entity;
pub mod error;
pub mod generator;
pub mod normalize;
pub mod predicate;
pub mod search;
pub mod validator;

pub use error::{Error, Result};
