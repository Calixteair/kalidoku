#!/usr/bin/env python3
"""
fame_score_rer.py

Computes a per-entity `fame_score` (0..=100) for the rer domain pack from
French Wikipedia pageviews.

Strategy
--------
WDQS rate-limits hard (1 req/min for repeat clients), so we skip Wikidata
entirely and go straight at the Wikipedia REST APIs:

1. For each entity, resolve a French Wikipedia article title by hitting
   `/api/rest_v1/page/summary/<candidate>` on each of:
      - "<name>" (raw)
      - "<name> (RER)"  — disambiguation suffix used by frwiki
      - "Gare de <name>"
   The first one that returns 200 wins. The summary endpoint follows
   redirects, so a hit on the raw name is canonicalised automatically.
2. For each resolved title, sum pageviews over the previous N days via
   `/api/rest_v1/metrics/pageviews/per-article/.../monthly/<start>/<end>`.
3. Percentile-rank the pageviews distribution → fame_score 0..=100. Ties
   share the average rank.

Output
------
- Patches `domains/rer/entities.json` in place.
- Writes an audit report at `domains/rer/tmp/fame_score_report.json` with
  per-entity match status, resolved title, pageviews, and final score.
- Bumps `metadata.version` and re-computes `metadata.dataset_sha256`.

Idempotency
-----------
Same Wikipedia snapshot → same scores. Re-running overrides previous
fame_scores; we don't preserve old values because the percentile ranks
shift whenever entities are added or removed.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Optional

# ---------- Constants ----------

PAGEVIEWS_BASE = "https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/fr.wikipedia/all-access/all-agents"
SUMMARY_BASE = "https://fr.wikipedia.org/api/rest_v1/page/summary"
USER_AGENT = "kalidoku-fame-ingest/1.0 (https://github.com/Calixteair/kalidoku contact: reymond.calixte@gmail.com)"

# Per-entity candidate title variants. Tried in order; first 200-OK wins.
# `(RER)` is the canonical disambiguator on fr.wikipedia for RER stations
# that share their name with a city or non-rail entity; `Gare de` covers
# main-line stops that get filed under the gare's article.
TITLE_VARIANTS = ["{name}", "{name} (RER)", "Gare de {name}"]


# ---------- HTTP ----------


def log(msg: str) -> None:
    print(f"[fame-rer] {msg}", file=sys.stderr, flush=True)


def http_get_json(url: str, retries: int = 3) -> Optional[dict]:
    """Returns None on 404 (not the 500 caller wants); raises on other errors."""
    req = urllib.request.Request(url)
    req.add_header("User-Agent", USER_AGENT)
    req.add_header("Accept", "application/json")
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                return json.loads(r.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None
            if e.code in (429, 500, 502, 503, 504) and attempt < retries - 1:
                wait = 2 ** attempt
                log(f"  HTTP {e.code} on {url[:80]}…, retry in {wait}s")
                time.sleep(wait)
                continue
            raise RuntimeError(f"HTTP {e.code} on {url}") from e
        except Exception as e:
            if attempt < retries - 1:
                time.sleep(2 ** attempt)
                continue
            raise RuntimeError(f"network error on {url}: {e}") from e
    return None


# ---------- Title resolution ----------


# Words we expect in the resolved article to plausibly be the station/gare
# article. Word-boundary anchored so 'métropole' (city stuff), 'commune',
# 'départemental' don't smuggle in unrelated pages — that was inflating
# Antony, Drancy, Aulnay etc. via redirects to their commune articles.
TOPIC_KEYWORDS_RE = __import__("re").compile(
    r"\b(?:gare|gares|station|stations|RER|m[ée]tro|m[ée]trop?olitain"
    r"|transilien|ferroviaire|tramway|chemin de fer)\b",
    __import__("re").IGNORECASE,
)


def looks_like_station(summary: dict) -> bool:
    desc = summary.get("description") or ""
    extract = (summary.get("extract") or "")[:600]
    text = f"{desc} {extract}"
    return bool(TOPIC_KEYWORDS_RE.search(text))


def resolve_title(entity_name: str) -> Optional[str]:
    """Try several frwiki title variants; return the canonical title that
    summary resolved to (follows redirects). Rejects pages whose summary
    doesn't look transit-related, otherwise homonyms (city, person, etc.)
    sneak in via redirect and inflate pageviews."""
    for variant in TITLE_VARIANTS:
        candidate = variant.format(name=entity_name)
        encoded = urllib.parse.quote(candidate.replace(" ", "_"), safe="()")
        url = f"{SUMMARY_BASE}/{encoded}"
        try:
            data = http_get_json(url)
        except RuntimeError as e:
            log(f"  summary error for '{candidate}': {e}")
            continue
        if data is None:
            continue
        if data.get("type") == "disambiguation":
            continue
        if not looks_like_station(data):
            # Wrong topic — try the next variant. The "Gare de X" variant
            # often rescues the match here (Issy → Issy-les-Moulineaux is
            # rejected, then "Gare d'Issy" or "Gare d'Issy-Val-de-Seine"
            # picks up the actual station article).
            continue
        title = data.get("title") or candidate
        return title
    return None


# ---------- Pageviews ----------


def fetch_pageviews_sum(title: str, start: str, end: str) -> int:
    """Sum monthly pageviews for `title` over [start, end] (YYYYMMDD).
    Returns 0 when the article exists but has no pageviews recorded."""
    encoded = urllib.parse.quote(title.replace(" ", "_"), safe="()")
    url = f"{PAGEVIEWS_BASE}/{encoded}/monthly/{start}/{end}"
    try:
        data = http_get_json(url)
    except RuntimeError:
        return 0
    if data is None:
        return 0
    return sum(item.get("views", 0) for item in data.get("items", []))


def percentile_rank(values: dict[str, int]) -> dict[str, int]:
    """Map each entity_id → percentile rank 0..=100 of its pageviews. Ties
    receive the average rank."""
    if not values:
        return {}
    items = sorted(values.items(), key=lambda kv: kv[1])
    n = len(items)
    ranks: dict[str, int] = {}
    i = 0
    while i < n:
        j = i
        while j + 1 < n and items[j + 1][1] == items[i][1]:
            j += 1
        rank_pct = ((i + j) / 2 + 0.5) / n * 100
        for k in range(i, j + 1):
            ranks[items[k][0]] = max(0, min(100, int(round(rank_pct))))
        i = j + 1
    return ranks


# ---------- Patch entities.json ----------


def patch_entities(entities: list[dict], fame_by_id: dict[str, int]) -> tuple[list[dict], int]:
    applied = 0
    out = []
    for ent in entities:
        copy = dict(ent)
        if ent["id"] in fame_by_id:
            copy["fame_score"] = fame_by_id[ent["id"]]
            applied += 1
        elif "fame_score" in copy:
            del copy["fame_score"]
        out.append(copy)
    return out, applied


def bump_metadata(meta: dict, dataset_bytes: bytes) -> dict:
    sha = hashlib.sha256(dataset_bytes).hexdigest()
    meta = dict(meta)
    meta["dataset_sha256"] = sha
    parts = meta["version"].split(".")
    if len(parts) == 3 and parts[1].isdigit():
        parts[1] = str(int(parts[1]) + 1)
        parts[2] = "0"
        meta["version"] = ".".join(parts)
    today = dt.date.today().isoformat()
    sv = meta.setdefault("sources_versions", [])
    sv = [s for s in sv if s.get("id") != "wikipedia-pageviews-fame"]
    sv.append(
        {
            "id": "wikipedia-pageviews-fame",
            "url": "https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/fr.wikipedia",
            "date": today,
        }
    )
    meta["sources_versions"] = sv
    return meta


def write_audit_report(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2))


# ---------- Main ----------


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1] if __doc__ else "")
    parser.add_argument(
        "--domain-root",
        type=Path,
        default=Path("domains/rer"),
        help="Path to the domain pack (default: domains/rer)",
    )
    parser.add_argument(
        "--days",
        type=int,
        default=365,
        help="Pageviews lookback window in days (default 365)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Compute and report but don't write entities.json / metadata.json",
    )
    args = parser.parse_args()

    domain_root: Path = args.domain_root
    entities_path = domain_root / "entities.json"
    meta_path = domain_root / "metadata.json"
    if not entities_path.is_file():
        log(f"missing {entities_path}")
        return 2

    entities = json.loads(entities_path.read_text())
    meta = json.loads(meta_path.read_text())
    log(f"loaded {len(entities)} entities from {entities_path}")

    end_date = dt.date.today().replace(day=1) - dt.timedelta(days=1)
    start_date = end_date - dt.timedelta(days=args.days)
    start_str = start_date.strftime("%Y%m01")
    end_str = end_date.strftime("%Y%m%d")
    log(f"fetching pageviews {start_str} → {end_str}")

    pv_by_id: dict[str, int] = {}
    audit = []
    matched = 0
    for i, ent in enumerate(entities, 1):
        record = {"id": ent["id"], "name": ent["name"]}
        title = resolve_title(ent["name"])
        if title is None:
            record["title"] = None
            record["pageviews"] = 0
            audit.append(record)
            continue
        matched += 1
        record["title"] = title
        try:
            pv = fetch_pageviews_sum(title, start_str, end_str)
        except Exception as e:
            log(f"  pageviews error for {ent['id']} ({title}): {e}")
            pv = 0
        pv_by_id[ent["id"]] = pv
        record["pageviews"] = pv
        audit.append(record)
        if i % 30 == 0:
            log(f"  {i}/{len(entities)} processed (matched={matched})")
        # Polite throttle — the Wikipedia REST endpoint is generous but
        # 150 ms between calls stays well under documented limits.
        time.sleep(0.15)

    fame_by_id = percentile_rank(pv_by_id)

    # Apply manual overrides last so a re-ingest doesn't undo curator
    # decisions. Used to patch outliers where the auto-resolver fell short
    # (Auber, CDG terminus, Antony / Issy commune redirects, etc.).
    overrides_path = domain_root / "fame_overrides.json"
    overrides: dict[str, int] = {}
    if overrides_path.is_file():
        try:
            data = json.loads(overrides_path.read_text())
            raw = data.get("overrides", {}) if isinstance(data, dict) else {}
            for k, v in raw.items():
                if isinstance(v, int) and 0 <= v <= 100:
                    overrides[k] = v
        except (json.JSONDecodeError, OSError) as e:
            log(f"  WARN: ignoring malformed fame_overrides.json: {e}")
    if overrides:
        log(f"  applying {len(overrides)} manual override(s)")
    for entity_id, score in overrides.items():
        fame_by_id[entity_id] = score

    for r in audit:
        r["fame_score"] = fame_by_id.get(r["id"])
        if r["id"] in overrides:
            r["override"] = True

    log(f"coverage: {matched}/{len(entities)} entities scored")

    if args.dry_run:
        log("dry-run: not writing files")
        report_path = domain_root / "tmp" / "fame_score_report.dryrun.json"
        write_audit_report(
            report_path,
            {
                "generated_at": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                "matched": matched,
                "total": len(entities),
                "audit": audit,
            },
        )
        log(f"audit report → {report_path}")
        return 0

    patched, applied = patch_entities(entities, fame_by_id)
    new_bytes = json.dumps(patched, ensure_ascii=False, indent=2).encode("utf-8") + b"\n"
    entities_path.write_bytes(new_bytes)
    log(f"wrote {entities_path} ({applied} entities scored)")

    new_meta = bump_metadata(meta, new_bytes)
    new_meta["entity_count"] = len(patched)
    meta_path.write_text(json.dumps(new_meta, ensure_ascii=False, indent=2) + "\n")
    log(f"wrote {meta_path} (version {new_meta['version']})")

    report_path = domain_root / "tmp" / "fame_score_report.json"
    write_audit_report(
        report_path,
        {
            "generated_at": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "window_start": start_str,
            "window_end": end_str,
            "matched": matched,
            "total": len(entities),
            "audit": audit,
        },
    )
    log(f"audit report → {report_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
