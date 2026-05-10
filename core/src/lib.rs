#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod domain;
pub mod entity;
pub mod error;
pub mod generator;
pub mod normalize;
pub mod predicate;
pub mod search;
pub mod validator;

pub use error::{Error, Result};
