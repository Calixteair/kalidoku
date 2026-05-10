//! Meilisearch reindexer.
//!
//! Reads every `domains/<id>/entities.json` (+ companion `metadata.json`)
//! from disk and pushes the documents to a Meilisearch instance.
//!
//! Design choices:
//! - **Raw HTTP** (`reqwest`) instead of an SDK : we already pull `reqwest`
//!   for OIDC, the API surface is tiny, and keeping the wire format explicit
//!   makes wiremock-based tests trivial.
//! - **One Meili index per domain** (`paris-metro`, `nyc-subway`, …). Index
//!   names are taken straight from each domain's `id`, which the JSON schema
//!   already constrains to `[a-z0-9][a-z0-9-]{0,63}` — Meili-compatible.
//! - **Idempotent by design** : Meilisearch deduplicates documents on the
//!   primary key (`id`), so re-running the command is always safe.
//! - **Best-effort task wait** : we poll `GET /tasks/{uid}` for up to
//!   `TASK_POLL_TIMEOUT`. A timeout is logged as `warn` but does not fail
//!   the whole run — Meili will eventually process the task.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const TASK_POLL_TIMEOUT: Duration = Duration::from_secs(30);
const TASK_POLL_INTERVAL: Duration = Duration::from_millis(250);
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// Stop-words list used by every domain. Kept inline (no extra crate dep) and
/// covers FR + EN function words that pollute autocomplete.
const STOP_WORDS: &[&str] = &[
    // FR
    "le", "la", "les", "un", "une", "des", "de", "du", "et", "ou", "à", "au", "aux", "en", "dans",
    "sur", "pour", "par", "avec", "sans", "que", "qui", "ce", "ces", "cet", "cette", "se", "sa",
    "son", "ses", "leur", "leurs", "ne", "pas", "plus", "très", "y", "il", "elle", "ils", "elles",
    "on", "nous", "vous", "je", "tu", // EN
    "the", "a", "an", "and", "or", "of", "in", "on", "at", "to", "for", "with", "without", "from",
    "by", "as", "is", "are", "was", "were", "be", "been", "this", "that", "these", "those", "it",
    "its", "i", "you", "he", "she", "we", "they",
];

/// Public entry point — discover every domain pack under `domains_dir` and
/// reindex each one in Meilisearch.
pub async fn reindex_all(meili_url: &str, meili_key: &str, domains_dir: &Path) -> Result<()> {
    if !domains_dir.exists() {
        tracing::warn!(
            path = %domains_dir.display(),
            "domains dir not found, skipping reindex"
        );
        return Ok(());
    }

    let client = Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .context("building reqwest client for Meilisearch")?;

    let packs = collect_packs(domains_dir)?;
    if packs.is_empty() {
        tracing::warn!(path = %domains_dir.display(), "no domain pack found, nothing to reindex");
        return Ok(());
    }

    let mut ok = 0_usize;
    let mut failed = 0_usize;
    for pack in packs {
        match reindex_one(&client, meili_url, meili_key, &pack).await {
            Ok(()) => ok += 1,
            Err(err) => {
                failed += 1;
                tracing::error!(domain = %pack.id, error = %err, "reindex failed");
            }
        }
    }
    tracing::info!(reindexed = ok, failed, "meilisearch reindex done");
    Ok(())
}

/// Locate every domain pack under `root`. Returns a sorted vector so the
/// reindex order is reproducible for tests / logs.
fn collect_packs(root: &Path) -> Result<Vec<PackOnDisk>> {
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
        let entities_path = path.join("entities.json");
        if !metadata_path.exists() || !entities_path.exists() {
            continue;
        }
        match read_pack(&path) {
            Ok(p) => out.push(p),
            Err(e) => tracing::warn!(
                path = %path.display(),
                error = %e,
                "ignoring invalid domain pack"
            ),
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

#[derive(Debug)]
struct PackOnDisk {
    id: String,
    documents: Vec<Value>,
}

#[derive(Deserialize)]
struct PackMeta {
    id: String,
}

fn read_pack(dir: &Path) -> Result<PackOnDisk> {
    let metadata_raw = std::fs::read(dir.join("metadata.json"))
        .with_context(|| format!("reading {}/metadata.json", dir.display()))?;
    let meta: PackMeta = serde_json::from_slice(&metadata_raw)
        .with_context(|| format!("parsing {}/metadata.json", dir.display()))?;

    let entities_raw = std::fs::read(dir.join("entities.json"))
        .with_context(|| format!("reading {}/entities.json", dir.display()))?;
    let entities: Vec<Value> = serde_json::from_slice(&entities_raw)
        .with_context(|| format!("parsing {}/entities.json", dir.display()))?;

    let documents = entities.into_iter().map(to_search_doc).collect();
    Ok(PackOnDisk {
        id: meta.id,
        documents,
    })
}

/// Flatten an entity into a Meili-friendly document.
///
/// Entities use a tagged-union shape (`{"lines": {"str_list": [...]}}`); Meili
/// indexes raw JSON, so we unwrap the inner value to keep `filterableAttributes`
/// usable without forcing the front-end to know about our internal envelope.
fn to_search_doc(entity: Value) -> Value {
    let mut doc = serde_json::Map::new();

    if let Some(obj) = entity.as_object() {
        if let Some(id) = obj.get("id") {
            doc.insert("id".into(), id.clone());
        }
        if let Some(name) = obj.get("name") {
            doc.insert("name".into(), name.clone());
        }
        if let Some(aliases) = obj.get("aliases") {
            doc.insert("aliases".into(), aliases.clone());
        }
        if let Some(Value::Object(attrs)) = obj.get("attributes") {
            for (k, v) in attrs {
                doc.insert(k.clone(), unwrap_attribute(v));
            }
        }
    }
    Value::Object(doc)
}

/// `{"str_list": ["12"]}` → `["12"]`, `{"num": 18}` → `18`, etc.
fn unwrap_attribute(v: &Value) -> Value {
    if let Some(obj) = v.as_object() {
        if obj.len() == 1 {
            if let Some((_, inner)) = obj.iter().next() {
                return inner.clone();
            }
        }
    }
    v.clone()
}

/// Settings sent in the `PATCH /indexes/{uid}/settings` body.
#[derive(Debug, Serialize)]
struct IndexSettings {
    #[serde(rename = "searchableAttributes")]
    searchable_attributes: Vec<String>,
    #[serde(rename = "filterableAttributes")]
    filterable_attributes: Vec<String>,
    synonyms: serde_json::Map<String, Value>,
    #[serde(rename = "stopWords")]
    stop_words: Vec<String>,
}

fn settings_for(documents: &[Value]) -> IndexSettings {
    let filterable: Vec<String> = documents
        .iter()
        .flat_map(|d| d.as_object().into_iter().flat_map(|m| m.keys()))
        .filter(|k| k.as_str() != "id" && k.as_str() != "name" && k.as_str() != "aliases")
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    IndexSettings {
        searchable_attributes: vec!["name".into(), "aliases".into()],
        filterable_attributes: filterable,
        synonyms: serde_json::Map::new(),
        stop_words: STOP_WORDS.iter().map(|s| (*s).to_owned()).collect(),
    }
}

async fn reindex_one(
    client: &Client,
    meili_url: &str,
    meili_key: &str,
    pack: &PackOnDisk,
) -> Result<()> {
    if pack.documents.is_empty() {
        tracing::warn!(domain = %pack.id, "empty entities.json, skipping reindex");
        return Ok(());
    }

    let base = meili_url.trim_end_matches('/');
    tracing::info!(
        domain = %pack.id,
        count = pack.documents.len(),
        "pushing documents to meilisearch"
    );

    let docs_url = format!("{base}/indexes/{}/documents?primaryKey=id", pack.id);
    let docs_resp = client
        .post(&docs_url)
        .bearer_auth(meili_key)
        .json(&pack.documents)
        .send()
        .await
        .with_context(|| format!("POST {docs_url}"))?;
    let docs_task = parse_task_response(docs_resp, "documents")
        .await
        .with_context(|| format!("documents push for '{}'", pack.id))?;
    wait_task(client, base, meili_key, docs_task).await;

    let settings = settings_for(&pack.documents);
    let settings_url = format!("{base}/indexes/{}/settings", pack.id);
    let settings_resp = client
        .patch(&settings_url)
        .bearer_auth(meili_key)
        .json(&settings)
        .send()
        .await
        .with_context(|| format!("PATCH {settings_url}"))?;
    let settings_task = parse_task_response(settings_resp, "settings")
        .await
        .with_context(|| format!("settings patch for '{}'", pack.id))?;
    wait_task(client, base, meili_key, settings_task).await;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct TaskAck {
    #[serde(rename = "taskUid")]
    task_uid: u64,
}

/// Validate Meili's HTTP response and pull the task uid out of the body.
/// Anything outside `2xx` is surfaced as an error so the caller can log it.
async fn parse_task_response(resp: reqwest::Response, kind: &str) -> Result<u64> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("meilisearch returned {status} on {kind}: {body}");
    }
    let ack: TaskAck = resp
        .json()
        .await
        .with_context(|| format!("decoding meilisearch {kind} ack"))?;
    Ok(ack.task_uid)
}

#[derive(Debug, Deserialize)]
struct TaskState {
    status: String,
}

/// Poll `GET /tasks/{uid}` until `succeeded` / `failed` / timeout.
/// Best-effort : a timeout is logged as warn but never propagates.
async fn wait_task(client: &Client, base: &str, meili_key: &str, task_uid: u64) {
    let url = format!("{base}/tasks/{task_uid}");
    let started = Instant::now();
    loop {
        if started.elapsed() >= TASK_POLL_TIMEOUT {
            tracing::warn!(task_uid, "meili task wait timed out, continuing");
            return;
        }
        match client.get(&url).bearer_auth(meili_key).send().await {
            Ok(resp) if resp.status() == StatusCode::OK => match resp.json::<TaskState>().await {
                Ok(state) => match state.status.as_str() {
                    "succeeded" => return,
                    "failed" | "canceled" => {
                        tracing::warn!(task_uid, status = %state.status, "meili task did not succeed");
                        return;
                    }
                    _ => {}
                },
                Err(err) => {
                    tracing::warn!(task_uid, error = %err, "decoding meili task body failed");
                    return;
                }
            },
            Ok(resp) => {
                tracing::warn!(task_uid, status = %resp.status(), "meili task lookup non-200");
                return;
            }
            Err(err) => {
                tracing::warn!(task_uid, error = %err, "meili task lookup transport error");
                return;
            }
        }
        tokio::time::sleep(TASK_POLL_INTERVAL).await;
    }
}

/// CLI entry point invoked from `--reindex`. Reads `MEILI_URL` and
/// `MEILI_MASTER_KEY` from the environment and uses
/// `KALIDOKU_DOMAINS_DIR` (default `domains/`) for pack discovery.
pub async fn run() -> Result<()> {
    let meili_url = read_env("MEILI_URL")?;
    let meili_key = read_env("MEILI_MASTER_KEY")?;
    let domains_dir = crate::domain_pack::default_root();
    tracing::info!(
        meili_url = %meili_url,
        domains = %domains_dir.display(),
        "kalidoku-worker --reindex starting"
    );
    reindex_all(&meili_url, &meili_key, &domains_dir).await
}

#[allow(clippy::disallowed_methods)] // worker has no central config service yet.
fn read_env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("env var {key} is required for --reindex"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;
    use wiremock::matchers::{method, path as path_matcher};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn write_pack(root: &Path, id: &str, entities: &Value) {
        let pack = root.join(id);
        fs::create_dir_all(&pack).unwrap();
        fs::write(
            pack.join("metadata.json"),
            format!(
                r#"{{"id":"{id}","name":{{"fr":"X"}},"version":"0.1.0","default_locale":"fr"}}"#
            ),
        )
        .unwrap();
        fs::write(
            pack.join("entities.json"),
            serde_json::to_string(entities).unwrap(),
        )
        .unwrap();
    }

    fn mini_entities() -> Value {
        json!([
            {
                "id": "abbesses",
                "name": "Abbesses",
                "attributes": {
                    "lines": {"str_list": ["12"]},
                    "in_paris": {"bool": true},
                    "arrondissement": {"num": 18}
                }
            },
            {
                "id": "alesia",
                "name": "Alésia",
                "aliases": ["Métro Alésia"],
                "attributes": {
                    "lines": {"str_list": ["4"]},
                    "in_paris": {"bool": true},
                    "arrondissement": {"num": 14}
                }
            }
        ])
    }

    #[tokio::test]
    async fn reindex_pushes_documents_and_settings() {
        let server = MockServer::start().await;
        let dir = tempdir().unwrap();
        write_pack(dir.path(), "paris-metro", &mini_entities());

        // Documents endpoint
        Mock::given(method("POST"))
            .and(path_matcher("/indexes/paris-metro/documents"))
            .respond_with(ResponseTemplate::new(202).set_body_json(json!({
                "taskUid": 1,
                "indexUid": "paris-metro",
                "status": "enqueued",
                "type": "documentAdditionOrUpdate"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Settings endpoint
        Mock::given(method("PATCH"))
            .and(path_matcher("/indexes/paris-metro/settings"))
            .respond_with(ResponseTemplate::new(202).set_body_json(json!({
                "taskUid": 2,
                "indexUid": "paris-metro",
                "status": "enqueued",
                "type": "settingsUpdate"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Task polling — return succeeded immediately.
        Mock::given(method("GET"))
            .and(path_matcher("/tasks/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "succeeded"})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_matcher("/tasks/2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "succeeded"})))
            .mount(&server)
            .await;

        reindex_all(&server.uri(), "test-key", dir.path())
            .await
            .expect("reindex_all should succeed");

        // Verify what hit the mock — primary key, doc shape, settings payload.
        let received = server.received_requests().await.unwrap();
        let docs_req: &Request = received
            .iter()
            .find(|r| r.method.as_str() == "POST" && r.url.path().contains("/documents"))
            .expect("documents request was sent");
        assert_eq!(
            docs_req.url.query_pairs().find(|(k, _)| k == "primaryKey"),
            Some(("primaryKey".into(), "id".into())),
            "primary key must be `id` so meili dedups on subsequent runs"
        );
        let docs_body: Vec<Value> = serde_json::from_slice(&docs_req.body).unwrap();
        assert_eq!(docs_body.len(), 2);
        assert_eq!(docs_body[0]["id"], "abbesses");
        assert_eq!(docs_body[0]["name"], "Abbesses");
        assert_eq!(docs_body[0]["lines"], json!(["12"]));
        assert_eq!(docs_body[0]["in_paris"], json!(true));
        assert_eq!(docs_body[0]["arrondissement"], json!(18));
        assert_eq!(docs_body[1]["aliases"], json!(["Métro Alésia"]));

        let settings_req: &Request = received
            .iter()
            .find(|r| r.method.as_str() == "PATCH" && r.url.path().contains("/settings"))
            .expect("settings request was sent");
        let settings_body: Value = serde_json::from_slice(&settings_req.body).unwrap();
        assert_eq!(
            settings_body["searchableAttributes"],
            json!(["name", "aliases"])
        );
        let filterable = settings_body["filterableAttributes"].as_array().unwrap();
        let filterable_set: BTreeSet<&str> =
            filterable.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(filterable_set.contains("lines"));
        assert!(filterable_set.contains("in_paris"));
        assert!(filterable_set.contains("arrondissement"));
        assert!(!filterable_set.contains("id"));
        assert!(!filterable_set.contains("name"));
        assert_eq!(settings_body["synonyms"], json!({}));
        let stop_words = settings_body["stopWords"].as_array().unwrap();
        assert!(!stop_words.is_empty(), "stop words must be set");
    }

    #[tokio::test]
    async fn missing_dir_is_a_warn_not_a_failure() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope");
        reindex_all("http://unused", "k", &missing).await.unwrap();
    }

    #[test]
    fn unwrap_attribute_strips_envelope() {
        assert_eq!(
            unwrap_attribute(&json!({"str_list": ["a", "b"]})),
            json!(["a", "b"])
        );
        assert_eq!(unwrap_attribute(&json!({"num": 42})), json!(42));
        assert_eq!(unwrap_attribute(&json!({"bool": true})), json!(true));
        // Pass-through when shape is not a single-key object.
        assert_eq!(unwrap_attribute(&json!("raw")), json!("raw"));
    }

    #[test]
    fn settings_filterable_excludes_identity_fields() {
        let docs = vec![json!({
            "id": "x",
            "name": "X",
            "aliases": [],
            "lines": ["1"],
            "in_paris": true,
        })];
        let s = settings_for(&docs);
        assert!(!s.filterable_attributes.contains(&"id".to_owned()));
        assert!(!s.filterable_attributes.contains(&"name".to_owned()));
        assert!(!s.filterable_attributes.contains(&"aliases".to_owned()));
        assert!(s.filterable_attributes.contains(&"lines".to_owned()));
        assert!(s.filterable_attributes.contains(&"in_paris".to_owned()));
    }
}
