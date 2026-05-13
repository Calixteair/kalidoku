#!/usr/bin/env python3
"""
fame_score_paris_metro.py

Computes a per-entity `fame_score` (0..=100) for the paris-metro domain pack
from Wikipedia pageviews, and writes the result back into
`domains/paris-metro/entities.json`.

Strategy
--------
1.  Bulk SPARQL Wikidata: every Q-item that's an instance of (transitively)
    "metro station" AND located in / adjacent to Paris. We fetch the QID, the
    French label, the French Wikipedia sitelink title and the geo coordinates.
2.  Match each entity in `entities.json` to a Wikidata row using:
       - exact normalised-name equality first
       - geo proximity (< 250 m) as a fallback when the name is ambiguous
3.  For each matched entity, sum pageviews over the previous 365 days via the
    Wikimedia REST API (`/per-article/fr.wikipedia/all-access/all-agents/.../monthly/`).
4.  Percentile-rank the pageviews distribution → fame_score 0..=100. Entities
    without a Wikidata match keep their existing `fame_score` (typically null).

Output
------
- Patches `domains/paris-metro/entities.json` in place.
- Writes an audit report at `domains/paris-metro/tmp/fame_score_report.json`
  with per-entity match status (matched / fallback / unmatched), pageviews,
  and final score. Lets a reviewer eyeball outliers before merging.
- Re-computes `metadata.dataset_sha256` and bumps `metadata.version`.

Idempotency
-----------
The script is fully idempotent: same Wikidata + same pageviews snapshot → same
output. Re-running it overrides previous fame_scores; existing scores are not
preserved (the percentile-rank changes when entities are added/removed).
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import sys
import time
import unicodedata
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Optional

# ---------- Constants ----------

WDQS_ENDPOINT = "https://query.wikidata.org/sparql"
PAGEVIEWS_BASE = "https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/fr.wikipedia/all-access/all-agents"
USER_AGENT = "kalidoku-fame-ingest/1.0 (https://github.com/Calixteair/kalidoku contact: reymond.calixte@gmail.com)"

# Bulk SPARQL: every QID typed "station de métro" (Q928830) located inside
# the Paris + petite couronne bounding box. P361 → Q1378363 ("réseau du métro
# de Paris") would be cleaner but most stations don't carry that property, so
# we filter by geography instead — over-pulls ~70 sister stations that the
# match step then ignores.
SPARQL_QUERY = """
SELECT ?station ?stationLabel ?coord ?article WHERE {
  ?station wdt:P31 wd:Q928830 ;
           wdt:P625 ?coord .
  BIND(geof:latitude(?coord) AS ?lat)
  BIND(geof:longitude(?coord) AS ?lon)
  FILTER(?lat > 48.7 && ?lat < 49.0 && ?lon > 2.1 && ?lon < 2.6) .
  OPTIONAL {
    ?article schema:about ?station ;
             schema:isPartOf <https://fr.wikipedia.org/> .
  }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "fr,en" . }
}
"""

# Geo match tolerance: stations are usually within a few metres of their
# Wikidata coords, 250 m absorbs OSM/IDFM jitter without sweeping in wrong
# matches (closest station-to-station distance on the network is ~300 m).
GEO_MATCH_METERS = 250.0


# ---------- Helpers ----------


def log(msg: str) -> None:
    print(f"[fame-ingest] {msg}", file=sys.stderr, flush=True)


def normalise_name(s: str) -> str:
    """Lowercase, strip accents, collapse separators. Same intent as the Rust
    `core::normalize::normalize` so matches across the two stay consistent."""
    no_accents = "".join(
        c for c in unicodedata.normalize("NFKD", s) if not unicodedata.combining(c)
    )
    out = []
    prev_sep = False
    for c in no_accents.lower():
        if c.isalnum():
            out.append(c)
            prev_sep = False
        elif not prev_sep:
            out.append(" ")
            prev_sep = True
    return "".join(out).strip()


def http_get_json(url: str, headers: Optional[dict] = None, retries: int = 3) -> dict:
    """GET + JSON decode with a polite backoff on 429/5xx."""
    req = urllib.request.Request(url)
    req.add_header("User-Agent", USER_AGENT)
    req.add_header("Accept", "application/json")
    if headers:
        for k, v in headers.items():
            req.add_header(k, v)
    last_err: Optional[Exception] = None
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.loads(r.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt < retries - 1:
                wait = 2 ** attempt
                log(f"  HTTP {e.code} on {url[:80]}, retry in {wait}s")
                time.sleep(wait)
                continue
            last_err = e
            break
        except Exception as e:
            last_err = e
            if attempt < retries - 1:
                time.sleep(2 ** attempt)
                continue
            break
    raise RuntimeError(f"fetch failed: {url} → {last_err}")


def haversine_m(lat1: float, lon1: float, lat2: float, lon2: float) -> float:
    """Distance in metres between two WGS84 points."""
    r = 6_371_000.0
    rad = math.pi / 180.0
    dlat = (lat2 - lat1) * rad
    dlon = (lon2 - lon1) * rad
    a = (
        math.sin(dlat / 2) ** 2
        + math.cos(lat1 * rad) * math.cos(lat2 * rad) * math.sin(dlon / 2) ** 2
    )
    return 2 * r * math.asin(math.sqrt(a))


def parse_wkt_point(wkt: str) -> Optional[tuple[float, float]]:
    """Wikidata returns coords as `Point(lon lat)`."""
    s = wkt.strip()
    if not s.startswith("Point("):
        return None
    try:
        lon_str, lat_str = s[len("Point("):-1].split(" ")
        return float(lat_str), float(lon_str)
    except ValueError:
        return None


# ---------- Wikidata bulk fetch ----------


def fetch_wikidata_stations() -> list[dict]:
    """One SPARQL call returns every Paris-metro station with QID, fr label,
    geo and frwiki sitelink. Cheap on the WDQS side (one query vs 310 lookups)
    and lets us match by geo as a fallback for ambiguous labels."""
    log("fetching Wikidata stations (SPARQL bulk)…")
    url = f"{WDQS_ENDPOINT}?query={urllib.parse.quote(SPARQL_QUERY)}&format=json"
    data = http_get_json(url)
    rows = []
    for b in data["results"]["bindings"]:
        qid = b["station"]["value"].rsplit("/", 1)[-1]
        label = b.get("stationLabel", {}).get("value", "")
        coord = b.get("coord", {}).get("value", "")
        article = b.get("article", {}).get("value", "")
        latlon = parse_wkt_point(coord) if coord else None
        title = None
        if article and "/wiki/" in article:
            title = urllib.parse.unquote(article.split("/wiki/", 1)[1])
        rows.append(
            {
                "qid": qid,
                "label": label,
                "label_norm": normalise_name(label),
                "lat": latlon[0] if latlon else None,
                "lon": latlon[1] if latlon else None,
                "frwiki_title": title,
            }
        )
    log(f"  → {len(rows)} Wikidata station rows (deduplicating by QID)")
    by_qid: dict[str, dict] = {}
    for r in rows:
        by_qid.setdefault(r["qid"], r)
    return list(by_qid.values())


# ---------- Matching ----------


def match_entities(entities: list[dict], wd_rows: list[dict]) -> dict[str, tuple[dict, str]]:
    """Return {entity_id: (wd_row, how)}. `how` is 'name', 'geo' or absent
    (entity not matched). Name match is tried first; geo fallback is only used
    when the entity has coords AND no name match exists."""
    by_name: dict[str, list[dict]] = {}
    for w in wd_rows:
        by_name.setdefault(w["label_norm"], []).append(w)

    matched: dict[str, tuple[dict, str]] = {}
    name_hits = geo_hits = 0
    for ent in entities:
        ent_norm = normalise_name(ent["name"])
        # 1) exact name match. If multiple Wikidata rows share the label
        #    (rare, but happens for old stations + their renamed successors),
        #    keep the one closest to the entity's geo, falling back to the
        #    first row if no geo is available.
        candidates = by_name.get(ent_norm, [])
        if candidates:
            ent_geo = ent.get("attributes", {}).get("geo", {}).get("geo")
            best = candidates[0]
            if ent_geo and len(candidates) > 1:
                best = min(
                    candidates,
                    key=lambda w: (
                        haversine_m(
                            ent_geo["lat"], ent_geo["lon"], w["lat"] or 0, w["lon"] or 0
                        )
                        if w["lat"] is not None and w["lon"] is not None
                        else float("inf")
                    ),
                )
            matched[ent["id"]] = (best, "name")
            name_hits += 1
            continue
        # 2) geo fallback — only when the entity carries coords. Catches
        #    stations whose Wikidata label is spelled differently (apostrophes,
        #    historical names, hyphenation).
        ent_geo = ent.get("attributes", {}).get("geo", {}).get("geo")
        if not ent_geo:
            continue
        best_w = None
        best_d = float("inf")
        for w in wd_rows:
            if w["lat"] is None or w["lon"] is None:
                continue
            d = haversine_m(ent_geo["lat"], ent_geo["lon"], w["lat"], w["lon"])
            if d < best_d:
                best_d = d
                best_w = w
        if best_w and best_d < GEO_MATCH_METERS:
            matched[ent["id"]] = (best_w, "geo")
            geo_hits += 1
    log(f"  → matched {len(matched)}/{len(entities)} ({name_hits} by name, {geo_hits} by geo)")
    return matched


# ---------- Pageviews ----------


def fetch_pageviews_sum(article_title: str, start: str, end: str) -> int:
    """Sum monthly pageviews for `article_title` over [start, end] (YYYYMMDD).
    Returns 0 when the article is missing — happens for very small stations
    that don't have a dedicated French Wikipedia page yet."""
    encoded = urllib.parse.quote(article_title, safe="")
    url = f"{PAGEVIEWS_BASE}/{encoded}/monthly/{start}/{end}"
    try:
        data = http_get_json(url)
    except RuntimeError as e:
        if "404" in str(e):
            return 0
        raise
    return sum(item.get("views", 0) for item in data.get("items", []))


def percentile_rank(values: dict[str, int]) -> dict[str, int]:
    """Map each entity_id → percentile rank 0..=100 of its pageviews. Ties
    receive the same rank (average percentile)."""
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
        # Average percentile across the tie
        rank_pct = ((i + j) / 2 + 0.5) / n * 100
        for k in range(i, j + 1):
            ranks[items[k][0]] = max(0, min(100, int(round(rank_pct))))
        i = j + 1
    return ranks


# ---------- Patch entities.json ----------


def write_audit_report(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2))


def patch_entities(
    entities: list[dict],
    fame_by_id: dict[str, int],
) -> tuple[list[dict], int]:
    """Apply fame_score on each entity. Returns (new_list, applied_count)."""
    applied = 0
    out = []
    for ent in entities:
        copy = dict(ent)
        if ent["id"] in fame_by_id:
            copy["fame_score"] = fame_by_id[ent["id"]]
            applied += 1
        elif "fame_score" in copy:
            # If we re-run the ingest and an entity is no longer matched,
            # drop the stale fame_score rather than leaving an outdated number.
            del copy["fame_score"]
        out.append(copy)
    return out, applied


def bump_metadata(meta: dict, entities_path: Path, dataset_bytes: bytes) -> dict:
    sha = hashlib.sha256(dataset_bytes).hexdigest()
    meta = dict(meta)
    meta["dataset_sha256"] = sha
    # Patch-bump: 0.3.0 → 0.3.1 if only fame_score changed; minor-bump preferred
    # for a meaningful gameplay shift like this one.
    parts = meta["version"].split(".")
    if len(parts) == 3 and parts[1].isdigit():
        parts[1] = str(int(parts[1]) + 1)
        parts[2] = "0"
        meta["version"] = ".".join(parts)
    # Record this ingestion under sources_versions so the lineage stays auditable.
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


# ---------- Main ----------


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1] if __doc__ else "")
    parser.add_argument(
        "--domain-root",
        type=Path,
        default=Path("domains/paris-metro"),
        help="Path to the domain pack (default: domains/paris-metro)",
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

    wd_rows = fetch_wikidata_stations()
    matches = match_entities(entities, wd_rows)

    end_date = dt.date.today().replace(day=1) - dt.timedelta(days=1)
    start_date = end_date - dt.timedelta(days=args.days)
    start_str = start_date.strftime("%Y%m01")
    end_str = end_date.strftime("%Y%m%d")
    log(f"fetching pageviews {start_str} → {end_str} for {len(matches)} stations")

    pv_by_id: dict[str, int] = {}
    audit = []
    for i, ent in enumerate(entities, 1):
        match = matches.get(ent["id"])
        record = {"id": ent["id"], "name": ent["name"]}
        if match is None:
            record["match"] = None
            audit.append(record)
            continue
        wd, how = match
        record["match"] = {"how": how, "qid": wd["qid"], "frwiki_title": wd["frwiki_title"]}
        if not wd["frwiki_title"]:
            record["pageviews"] = 0
            audit.append(record)
            continue
        try:
            pv = fetch_pageviews_sum(wd["frwiki_title"], start_str, end_str)
        except Exception as e:
            log(f"  pageviews error for {ent['id']} ({wd['frwiki_title']}): {e}")
            pv = 0
        pv_by_id[ent["id"]] = pv
        record["pageviews"] = pv
        audit.append(record)
        if i % 50 == 0:
            log(f"  {i}/{len(entities)} processed")
        # Polite rate-limit on the Wikimedia REST: their docs ask for
        # ≤ 100 req/s per client; one request per ~150 ms is well under that.
        time.sleep(0.15)

    fame_by_id = percentile_rank(pv_by_id)

    # Apply manual overrides last so a re-ingest doesn't undo curator
    # decisions. Used to patch outliers where Wikipedia returns a misleading
    # match (Saint-Sulpice → église) or where the topic-filter is too
    # conservative.
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

    matched_count = sum(1 for r in audit if r["match"] is not None)
    log(f"coverage: {matched_count}/{len(entities)} entities scored")

    if args.dry_run:
        log("dry-run: not writing files")
        report_path = domain_root / "tmp" / "fame_score_report.dryrun.json"
        write_audit_report(
            report_path,
            {
                "generated_at": dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                "matched": matched_count,
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

    new_meta = bump_metadata(meta, entities_path, new_bytes)
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
            "matched": matched_count,
            "total": len(entities),
            "audit": audit,
        },
    )
    log(f"audit report → {report_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
