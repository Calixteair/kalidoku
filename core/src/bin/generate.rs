//! CLI entry point: `kalidoku-generate --domain paris-metro --seed 2026-05-10`
//!
//! Owned by agent A. Loads a domain pack from disk, runs `core::generator::generate`,
//! prints the resulting grid as JSON on stdout.

fn main() -> anyhow::Result<()> {
    eprintln!("agent-A: implement CLI (see docs/agents/agent-a-core.md §5)");
    Ok(())
}
