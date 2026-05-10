//! Filesystem-based domain discovery.
//!
//! When the `domains` table isn't reachable (early MVP, agent C migrations
//! still pending), we fall back to listing `domains/*/metadata.json` directly
//! so the worker remains usable in dev / CI.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Minimal metadata projection — we only need `id` to locate the pack on disk.
#[derive(Debug, Clone, Deserialize)]
pub struct PackMetadata {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredDomain {
    pub id: String,
    pub root: PathBuf,
}

/// List every `domains/<id>/metadata.json` under `root` (typically the
/// `domains/` directory at the repo root).
///
/// Hidden directories and files that don't deserialize as `PackMetadata`
/// are skipped with a warning rather than failing the whole discovery.
pub fn discover(root: &Path) -> Result<Vec<DiscoveredDomain>> {
    if !root.exists() {
        tracing::warn!(path = %root.display(), "domain root does not exist, skipping fs discovery");
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(root).with_context(|| format!("read_dir({})", root.display()))?;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "skipping unreadable directory entry");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let metadata_path = path.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }
        match read_metadata(&metadata_path) {
            Ok(meta) => out.push(DiscoveredDomain {
                id: meta.id,
                root: path,
            }),
            Err(e) => tracing::warn!(
                path = %metadata_path.display(),
                error = %e,
                "ignoring invalid domain metadata"
            ),
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn read_metadata(path: &Path) -> Result<PackMetadata> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let meta: PackMetadata =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(meta)
}

/// Resolve the default `domains/` location for the current process.
///
/// Honours the `KALIDOKU_DOMAINS_DIR` env var when set, otherwise falls back
/// to `<cwd>/domains`. Used by `--once` and `--cron`.
#[allow(clippy::disallowed_methods)] // worker has no central config service yet.
pub fn default_root() -> PathBuf {
    std::env::var("KALIDOKU_DOMAINS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("domains"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn discover_lists_packs() {
        let dir = tempdir().unwrap();
        let pack = dir.path().join("paris-metro");
        fs::create_dir_all(&pack).unwrap();
        fs::write(
            pack.join("metadata.json"),
            r#"{"id":"paris-metro","name":{"fr":"Métro"},"version":"0.1.0","default_locale":"fr"}"#,
        )
        .unwrap();

        let got = discover(dir.path()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "paris-metro");
    }

    #[test]
    fn discover_skips_non_domain_dirs() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("not-a-domain")).unwrap();
        let ok = dir.path().join("ok");
        fs::create_dir_all(&ok).unwrap();
        fs::write(
            ok.join("metadata.json"),
            r#"{"id":"ok","name":{"fr":"OK"},"version":"0.1.0","default_locale":"fr"}"#,
        )
        .unwrap();

        let got = discover(dir.path()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "ok");
    }

    #[test]
    fn discover_returns_empty_when_root_missing() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let got = discover(&missing).unwrap();
        assert!(got.is_empty());
    }
}
