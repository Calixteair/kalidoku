//! kalidoku-worker library entry point.
//!
//! Exposing the worker modules through a `lib.rs` lets integration tests
//! (`tests/`) drive the same generation pipeline used by the `--once` and
//! `--cron` modes without going through a subprocess.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

pub mod cli;
pub mod cron;
pub mod db;
pub mod domain_pack;
pub mod once;
pub mod persist;
pub mod queue;
pub mod reindex;
pub mod seed;
pub mod telemetry;
