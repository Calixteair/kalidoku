//! CLI entry point: `kalidoku-generate --domain ./domains/paris-metro --seed 2026-05-10`.
//!
//! Loads a domain pack from disk, runs `core::generator::generate`, prints the resulting
//! grid as JSON on stdout. Exit code is non-zero on any error so this binary plugs into
//! shell pipelines without surprises.

use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use kalidoku_core::{
    domain::load_domain_pack,
    generator::{generate, GenerationOptions},
};

#[derive(Debug, Parser)]
#[command(
    name = "kalidoku-generate",
    about = "Generate a kalidoku 3x3 grid from a domain pack.",
    version
)]
struct Args {
    /// Path to the domain pack directory (must contain metadata.json, predicates.json, entities.json).
    #[arg(long)]
    domain: PathBuf,

    /// Seed used for reproducible generation. Accepts an integer or a free string
    /// (hashed to u64 via FNV-1a so e.g. `--seed 2026-05-10` works as expected).
    #[arg(long)]
    seed: Option<String>,

    /// Maximum number of attempts before the generator gives up.
    #[arg(long, default_value_t = 200)]
    max_attempts: u32,

    /// Pretty-print the JSON output (indented, multi-line).
    #[arg(long)]
    pretty: bool,

    /// Locale used to render predicate labels in the JSON output.
    #[arg(long, default_value = "fr")]
    locale: String,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kalidoku-generate: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let domain = load_domain_pack(&args.domain).map_err(|e| e.to_string())?;
    let seed = args
        .seed
        .as_deref()
        .map_or_else(|| GenerationOptions::default().seed, hash_seed);
    let opts = GenerationOptions {
        seed,
        max_attempts: args.max_attempts,
    };
    let grid = generate(&domain, opts).map_err(|e| e.to_string())?;
    let snapshot = grid.snapshot(&args.locale, seed);
    let json = if args.pretty {
        serde_json::to_string_pretty(&snapshot)
    } else {
        serde_json::to_string(&snapshot)
    }
    .map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(())
}

/// Convert any string into a stable u64 seed. Plain integers still go through this
/// function but produce the same number, since FNV is identity on already-numeric input.
fn hash_seed(raw: &str) -> u64 {
    if let Ok(n) = raw.parse::<u64>() {
        return n;
    }
    // FNV-1a 64-bit
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
