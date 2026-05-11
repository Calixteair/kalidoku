//! Server-side grid generator for solo mode.
//!
//! Daily grids are produced by the worker (cron) and persisted ahead of time.
//! Solo is per-player, on-demand and unlimited, so we generate at the request
//! latency. The generator itself is pure CPU and runs in <100 ms for the
//! paris-metro pack, so we don't queue it through the worker — calling
//! `core::generator::generate` directly inside a `spawn_blocking` keeps the
//! latency floor cheap and avoids a Redis hop in the hot path.
//!
//! The domain pack files (`metadata.json`, `predicates.json`, `entities.json`)
//! are baked into the server image at `/app/domains/<id>/` by Dockerfile.server
//! — see infra/docker/Dockerfile.server.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use kalidoku_core::{
    domain::{load_domain_pack, Domain},
    generator::{generate, GenerationOptions},
};
use rand::{rngs::OsRng, RngCore};

/// Pair `(payload, version, seed)` returned by the generator.
pub struct SoloGrid {
    pub payload: serde_json::Value,
    pub version: String,
    pub seed: u64,
}

/// Same defaults as the worker: 500 attempts is well above the empirical
/// success rate (paris-metro v0.3 ~100% in <10 attempts).
const GENERATOR_MAX_ATTEMPTS: u32 = 500;

/// Generate a brand-new solo grid for `domain_id`. When `seed` is `None` we
/// draw a fresh 32-bit value so the seed stays short enough to share in a URL
/// (`?seed=314159`). 32 bits = 4 billion possibilities, plenty for shuffling.
///
/// `domains_root` is the filesystem root holding the packs — typically the
/// config-provided `cfg.domains_root` (`/app/domains` in container, `domains`
/// in dev). Tests pass an absolute path to the workspace `domains/` dir.
pub async fn generate_solo(
    domains_root: &Path,
    domain_id: &str,
    seed: Option<u64>,
) -> Result<SoloGrid> {
    let chosen_seed = seed.unwrap_or_else(random_short_seed);
    let id = domain_id.to_owned();
    let root: PathBuf = domains_root.join(&id);

    tokio::task::spawn_blocking(move || run_blocking(&id, &root, chosen_seed))
        .await
        .context("solo generator task crashed")?
}

fn run_blocking(
    domain_id: &str,
    domain_root: &std::path::Path,
    seed_value: u64,
) -> Result<SoloGrid> {
    let pack: Domain = load_domain_pack(domain_root).with_context(|| {
        format!(
            "loading domain pack '{domain_id}' from {}",
            domain_root.display()
        )
    })?;

    let opts = GenerationOptions {
        seed: seed_value,
        max_attempts: GENERATOR_MAX_ATTEMPTS,
    };
    let grid =
        generate(&pack, opts).with_context(|| format!("generating solo grid for '{domain_id}'"))?;

    let locale = pack.metadata.default_locale.as_str();
    let snapshot = grid.snapshot_with_entities(locale, seed_value, &pack.entities);

    Ok(SoloGrid {
        payload: serde_json::to_value(snapshot).context("serialising grid snapshot")?,
        version: pack.metadata.version.clone(),
        seed: seed_value,
    })
}

/// 32 random bits, kept short so the value stays comfortable to share in a URL
/// or read aloud (e.g. "seed 314159"). 4 billion possibilities is plenty given
/// the deterministic CSP search is the actual entropy source.
fn random_short_seed() -> u64 {
    let mut buf = [0u8; 4];
    OsRng.fill_bytes(&mut buf);
    u64::from(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_domains_root() -> PathBuf {
        let manifest = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest)
            .parent()
            .expect("workspace root")
            .join("domains")
    }

    #[tokio::test]
    async fn generates_a_solo_for_paris_metro() {
        let root = workspace_domains_root();
        if !root.join("paris-metro").exists() {
            eprintln!(
                "skipping: paris-metro pack not present at {}",
                root.display()
            );
            return;
        }
        let g = generate_solo(&root, "paris-metro", Some(42)).await.unwrap();
        assert_eq!(g.seed, 42);
        assert!(g.payload.get("rows").is_some());
        assert!(g.payload.get("cols").is_some());
        assert!(g.payload.get("candidates").is_some());
    }

    #[tokio::test]
    async fn random_seed_is_short() {
        let s = random_short_seed();
        assert!(s < (1u64 << 32));
    }
}
