#!/usr/bin/env python3
"""
build_rer_dataset_v2.py — rebuild `domains/rer/entities.json` from the
French Wikipedia category tree of the RER d'Île-de-France lines.

Why v2
------
v1 (build_rer_dataset.py) pulled stations from OSM Overpass relations
filtered on `network="RER"` with role `stop` only. Both filters are too
strict on the live OSM data:
  - many SNCF-operated portions tag `network` as `Transilien` or
    `SNCF`/`Île-de-France;RER`, so whole branches disappear;
  - several stations are mapped with role `halt`/`station`/empty, which
    is silently dropped;
  - corrupt `name` tags (e.g. "Essonnes Robinson") slip through because
    nothing cross-checks the station against a canonical source.

The result was ~230 stations including phantom ones and missing terminals
like Saint-Germain-en-Laye, Poissy, Cergy-le-Haut, Marne-la-Vallée Chessy,
Boissy-Saint-Léger.

v2 takes the canonical station list from frwiki categories
"Ligne <A..E> du RER d'Île-de-France", filters titles that look like a
station page, and pulls metadata via the Wikipedia REST summary endpoint
(name, coords, wikibase QID). Wikidata SPARQL is intentionally avoided
because WDQS has been rate-limited to ~1 req/min since the 2024 outage
and routinely returns Wikimedia error HTML for category-scale workloads.

Reconciliation
--------------
For each station we compute a canonical id by stripping the "Gare de(s)?"
prefix from the article title and normalising it. When the same id already
exists in the previous entities.json, we keep it as-is so that
`fame_overrides.json` and previously generated grids referencing that id
stay valid. Otherwise we mint a fresh `normalise_id(name)` id.

Outputs
-------
- `domains/rer/entities.json` (sorted by id)
- `domains/rer/metadata.json` (version bump, sha256, sources_versions)
- `domains/rer/tmp/build_v2_diff.json` (dry-run only)

The script defaults to dry-run. Pass `--apply` to write.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import unicodedata
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Optional

REPO_ROOT = Path(__file__).resolve().parents[2]
DOMAIN_DIR_DEFAULT = REPO_ROOT / "domains" / "rer"

USER_AGENT = (
    "kalidoku-rer-ingest/2.0 "
    "(https://github.com/Calixteair/kalidoku contact: reymond.calixte@gmail.com)"
)

FRWIKI_API = "https://fr.wikipedia.org/w/api.php"
FRWIKI_REST_SUMMARY = "https://fr.wikipedia.org/api/rest_v1/page/summary"
OVERPASS_ENDPOINT = "https://overpass-api.de/api/interpreter"

LINES = ("A", "B", "C", "D", "E")
CATEGORY_TEMPLATE = "Catégorie:Ligne {letter} du RER d'Île-de-France"

# Paris bbox (inner ring road). Same heuristic as v1 for in_paris.
PARIS_BBOX = (48.815, 2.224, 48.902, 2.470)

# Cache TTL for frwiki responses: 7 days. Wikipedia REST is not rate
# limited but staying offline on repeat runs is friendlier.
CACHE_TTL_SECONDS = 7 * 86400


# ---------- Logging ----------


def log(msg: str) -> None:
    print(f"[rer-v2] {msg}", file=sys.stderr, flush=True)


# ---------- HTTP ----------


def http_get_json(url: str, retries: int = 3, body: Optional[bytes] = None) -> dict:
    req = urllib.request.Request(url, data=body)
    req.add_header("User-Agent", USER_AGENT)
    req.add_header("Accept", "application/json")
    if body is not None:
        req.add_header("Content-Type", "application/x-www-form-urlencoded")
    last_err: Optional[Exception] = None
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                return json.loads(r.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt < retries - 1:
                wait = 2 ** attempt
                log(f"  HTTP {e.code} for {url[:120]}…, retry in {wait}s")
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
    raise RuntimeError(f"fetch failed [{url[:120]}]: {last_err}")


def cached_json(cache_path: Path, url: str) -> dict:
    if cache_path.exists():
        age = time.time() - cache_path.stat().st_mtime
        if age < CACHE_TTL_SECONDS:
            return json.loads(cache_path.read_text(encoding="utf-8"))
    data = http_get_json(url)
    cache_path.parent.mkdir(parents=True, exist_ok=True)
    cache_path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
    return data


# ---------- Wikipedia queries ----------


def fetch_category_members(letter: str, cache_dir: Path) -> list[str]:
    """Return frwiki page titles in category `Ligne <letter> du RER…`."""
    title = CATEGORY_TEMPLATE.format(letter=letter)
    url = (
        f"{FRWIKI_API}?action=query&list=categorymembers"
        f"&cmtitle={urllib.parse.quote(title)}"
        f"&cmlimit=500&cmtype=page&format=json"
    )
    cache_path = cache_dir / f"category-rer-{letter}.json"
    data = cached_json(cache_path, url)
    members = data.get("query", {}).get("categorymembers", [])
    return [m["title"] for m in members]


# Titles we keep as station pages. Most are "Gare de(s)? Xxx" / "Gare d'Xxx" /
# "Gare du Xxx". A handful of recent stations use the bare "Gare Xxx-Yyy"
# pattern on frwiki (e.g. "Gare Rosa-Parks", "Gare Pierrefitte - Stains").
# The two terminus stations of Paris are exceptional: "Paris-Gare-de-Lyon".
_STATION_TITLE_RE = re.compile(
    r"^(?:"
    r"Gare\s+(?:de\s+|d['\u2019]|du\s+|des\s+)"
    r"|Paris-Gare-de-"
    r"|Gare\s+[A-ZÀ-Ý]"  # "Gare Rosa-Parks", "Gare Pierrefitte-Stains"
    r")",
    re.IGNORECASE | re.UNICODE,
)


def looks_like_station(title: str) -> bool:
    return bool(_STATION_TITLE_RE.match(title))


def fetch_summary(title: str, cache_dir: Path) -> Optional[dict]:
    url = f"{FRWIKI_REST_SUMMARY}/{urllib.parse.quote(title.replace(' ', '_'))}"
    # Filesystem-safe cache filename.
    safe = re.sub(r"[^A-Za-z0-9._-]+", "_", title)[:120]
    cache_path = cache_dir / f"summary-{safe}.json"
    try:
        return cached_json(cache_path, url)
    except RuntimeError as e:
        log(f"  summary miss for {title!r}: {e}")
        return None


# ---------- OSM ground-truth name lookup ----------


_OSM_NAME_NOISE_RE = [
    # "Aéroport Charles de Gaulle 1 (Terminal 3)" → drop the (Terminal …).
    re.compile(r"\s*\(Terminal[^)]*\)\s*$", re.IGNORECASE),
    # "Pierrefitte - Stains T11" → drop trailing tram-line suffix.
    re.compile(r"\s+[-–—]?\s*T\d{1,2}\s*$"),
    # "Saint-Denis - Gare" → the trailing "Gare" is OSM-internal noise.
    re.compile(r"\s+[-–—]\s*Gare\s*$", re.IGNORECASE),
]


def clean_osm_name(raw: str) -> str:
    out = raw
    for pat in _OSM_NAME_NOISE_RE:
        out = pat.sub("", out).strip()
    return out


def fetch_osm_names_by_qid(qids: list[str], cache_dir: Path) -> dict[str, str]:
    """One Overpass call → {qid: ground-truth `name` tag from OSM}.

    OSM tags `wikidata=Q...` on the station node/way, so we can pull the
    operator-facing name (RATP/SNCF signage) without going through WDQS.
    For QIDs with multiple OSM matches (the same station mapped both as a
    node and a way is common), we pick the first non-empty `name`.

    Stations that aren't tagged with their QID on OSM yield no entry — the
    caller falls back to the frwiki-derived name.
    """
    if not qids:
        return {}
    cache_path = cache_dir / "osm-names-by-qid.json"
    cached_qids: set[str] = set()
    if cache_path.exists():
        age = time.time() - cache_path.stat().st_mtime
        if age < CACHE_TTL_SECONDS:
            stored = json.loads(cache_path.read_text(encoding="utf-8"))
            cached_qids = set(stored.get("qids", []))
            # Cache key must match exactly the QID set: any drift = re-fetch.
            if cached_qids == set(qids):
                return stored["names"]

    # Overpass query: union of `nwr[wikidata=Q...]` for every QID. With 240
    # QIDs the URL hits 414; pass via POST body instead.
    selectors = "\n  ".join(f'nwr["wikidata"="{q}"];' for q in qids)
    query = f"[out:json][timeout:120];\n(\n  {selectors}\n);\nout tags center;"
    log(f"fetching OSM ground-truth names for {len(qids)} QIDs via Overpass…")
    body = urllib.parse.urlencode({"data": query}).encode("utf-8")
    data = http_get_json(OVERPASS_ENDPOINT, body=body)
    names: dict[str, str] = {}
    for el in data.get("elements", []):
        tags = el.get("tags") or {}
        qid = tags.get("wikidata")
        if not qid:
            continue
        # Prefer the `name` tag (operator signage). `official_name` is rarer
        # and on RER mostly duplicates `name`. `name:fr` is irrelevant —
        # everything in IDF is French already.
        n = tags.get("name") or tags.get("official_name") or ""
        n = clean_osm_name(n.strip())
        if not n:
            continue
        existing = names.get(qid)
        # If we already kept one for this QID, prefer the one tagged
        # `railway=station` (canonical station vs platform/halt clutter).
        if existing and tags.get("railway") != "station":
            continue
        names[qid] = n
    log(f"  → OSM matched {len(names)}/{len(qids)} QIDs")
    cache_path.write_text(
        json.dumps({"qids": sorted(qids), "names": names}, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return names


# ---------- Canonicalisation ----------


# Mapping article-aware: the frwiki convention is "Gare de Xxx" where the
# article (le/la/les/l') belongs to the station name and must be kept.
# So "Gare du Bourget" expands to "Le Bourget", not "Bourget".
_CANONICAL_PREFIX_PATTERNS: list[tuple[re.Pattern, str]] = [
    (re.compile(r"^Gare\s+de\s+la\s+", re.IGNORECASE | re.UNICODE), "La "),
    (re.compile(r"^Gare\s+de\s+l['\u2019]", re.IGNORECASE | re.UNICODE), "L'"),
    (re.compile(r"^Gare\s+du\s+", re.IGNORECASE | re.UNICODE), "Le "),
    (re.compile(r"^Gare\s+des\s+", re.IGNORECASE | re.UNICODE), "Les "),
    (re.compile(r"^Gare\s+d['\u2019]", re.IGNORECASE | re.UNICODE), ""),
    (re.compile(r"^Gare\s+de\s+", re.IGNORECASE | re.UNICODE), ""),
]


def canonical_name(title: str) -> str:
    """Turn a frwiki title into the station's human name.

    The frwiki article "Gare <article> Xxx" encodes the gare's article in
    the prefix: stripping the prefix without restoring the article would
    give "Bourget" instead of "Le Bourget", "Défense" instead of "La
    Défense" — visible on the in-game grid. So we map each prefix variant
    to its article and preserve it.

    Examples:
        "Gare de Saint-Germain-en-Laye" → "Saint-Germain-en-Laye"
        "Gare d'Auber" → "Auber"
        "Gare du Bourget" → "Le Bourget"
        "Gare de la Défense" → "La Défense"
        "Gare des Ardoines" → "Les Ardoines"
        "Gare de l'Étang" → "L'Étang"
        "Paris-Gare-de-Lyon" → "Gare de Lyon"
    """
    if title.startswith("Paris-Gare-de-"):
        rest = title[len("Paris-") :].replace("-", " ").strip()
        return rest
    stripped = title
    matched = False
    for pattern, prefix in _CANONICAL_PREFIX_PATTERNS:
        new_stripped, n = pattern.subn(prefix, title, count=1)
        if n:
            stripped = new_stripped
            matched = True
            break
    if not matched:
        # "Gare Rosa-Parks" → "Rosa Parks", "Gare Pierrefitte-Stains" → "Pierrefitte Stains".
        # Dashes are SEO-style on frwiki titles, restore the spaces.
        m = re.match(r"^Gare\s+(?P<rest>[A-ZÀ-Ý].*)$", title, re.UNICODE)
        if m:
            stripped = m.group("rest").replace("-", " ")
    stripped = re.sub(r"\s*\((?:RATP|SNCF|RER)\)\s*$", "", stripped).strip()
    return stripped or title


def normalise_id(name: str) -> str:
    s = unicodedata.normalize("NFKD", name)
    s = "".join(c for c in s if not unicodedata.combining(c))
    s = s.lower()
    s = re.sub(r"[^a-z0-9]+", "-", s)
    s = s.strip("-")
    return s[:64] or "x"


def in_paris(lat: float, lon: float) -> bool:
    return PARIS_BBOX[0] <= lat <= PARIS_BBOX[2] and PARIS_BBOX[1] <= lon <= PARIS_BBOX[3]


# ---------- Pipeline ----------


def collect_per_line(cache_dir: Path) -> dict[str, list[str]]:
    """For each RER line, return the list of station-looking frwiki titles."""
    per_line: dict[str, list[str]] = {}
    for letter in LINES:
        all_titles = fetch_category_members(letter, cache_dir)
        kept = [t for t in all_titles if looks_like_station(t)]
        log(f"line {letter}: {len(kept)}/{len(all_titles)} category pages look like a station")
        per_line[letter] = kept
    return per_line


def build_entities(
    per_line: dict[str, list[str]],
    previous_ids_by_qid: dict[str, str],
    previous_ids_by_name: dict[str, str],
    cache_dir: Path,
) -> tuple[list[dict], dict[str, str]]:
    """Resolve each title to a summary, dedup by QID, emit entities.

    Returns (entities, wiki_titles) where wiki_titles maps each entity_id
    to its frwiki article title — consumed by fame_score_rer.py to skip
    its title-guessing step.
    """
    by_qid: dict[str, dict] = {}
    no_qid: list[dict] = []
    skipped_no_geo = 0
    skipped_no_summary = 0

    for letter, titles in per_line.items():
        for title in titles:
            summary = fetch_summary(title, cache_dir)
            if not summary:
                skipped_no_summary += 1
                continue
            coords = summary.get("coordinates")
            if not coords:
                # No geo = useless for the geo predicates. Drop.
                skipped_no_geo += 1
                continue
            qid = summary.get("wikibase_item")
            name = canonical_name(summary.get("titles", {}).get("canonical", title).replace("_", " "))

            target = by_qid.get(qid) if qid else None
            if target is None and qid:
                target = {
                    "name": name,
                    "qid": qid,
                    "lat": coords["lat"],
                    "lon": coords["lon"],
                    "lines": set(),
                    "frwiki_title": title,
                }
                by_qid[qid] = target
            if target is None:
                # No QID at all: index by name in a parallel list.
                existing = next((x for x in no_qid if x["name"] == name), None)
                if existing is None:
                    existing = {
                        "name": name,
                        "qid": None,
                        "lat": coords["lat"],
                        "lon": coords["lon"],
                        "lines": set(),
                        "frwiki_title": title,
                    }
                    no_qid.append(existing)
                target = existing
            target["lines"].add(letter)

    log(
        f"resolved {len(by_qid)} stations by QID, {len(no_qid)} by name-only "
        f"(skipped {skipped_no_geo} no-coords, {skipped_no_summary} no-summary)"
    )

    # OSM ground truth: prefer the `name` tag on OSM (RATP/SNCF signage)
    # over the canonical_name() heuristic, which can't disambiguate when
    # frwiki keeps the "Gare du" prefix on stations whose commercial name
    # has no article ("Le Luxembourg" vs "Luxembourg").
    osm_names = fetch_osm_names_by_qid(sorted(by_qid.keys()), cache_dir)
    overrides = 0
    for qid, record in by_qid.items():
        ground = osm_names.get(qid)
        if ground and ground != record["name"]:
            record["name"] = ground
            overrides += 1
    log(f"  → {overrides} station names overridden from OSM ground truth")

    out: list[dict] = []
    wiki_titles: dict[str, str] = {}
    used_ids: set[str] = set()
    for record in [*by_qid.values(), *no_qid]:
        ent = record_to_entity(record, previous_ids_by_qid, previous_ids_by_name, used_ids)
        out.append(ent)
        wiki_titles[ent["id"]] = record["frwiki_title"]
    out.sort(key=lambda e: e["id"])
    return out, wiki_titles


def record_to_entity(
    record: dict,
    previous_ids_by_qid: dict[str, str],
    previous_ids_by_name: dict[str, str],
    used_ids: set[str],
) -> dict:
    """Mint an entity dict from an intermediate record, preferring stable
    ids when the QID or canonical name was already in the prior dataset."""
    name: str = record["name"]
    qid: Optional[str] = record["qid"]

    chosen_id: Optional[str] = None
    if qid and qid in previous_ids_by_qid:
        chosen_id = previous_ids_by_qid[qid]
    if chosen_id is None and name in previous_ids_by_name:
        chosen_id = previous_ids_by_name[name]
    if chosen_id is None:
        chosen_id = normalise_id(name)

    # Resolve id conflicts (very rare given QID stability + name stability).
    final = chosen_id
    n = 1
    while final in used_ids:
        n += 1
        final = f"{chosen_id}-{n}"
    used_ids.add(final)

    lat = float(record["lat"])
    lon = float(record["lon"])
    return {
        "id": final,
        "name": name,
        "attributes": {
            "lines": {"str_list": sorted(record["lines"])},
            "geo": {"geo": {"lat": lat, "lon": lon}},
            "in_paris": {"bool": in_paris(lat, lon)},
            # zone tarifaire: still not auto-discoverable. Set to 0.
            # A follow-up enrich pass could read OSM `network:wikipedia` /
            # `zone` tags or Wikidata P5031 once WDQS is workable again.
            "zone": {"num": 0.0},
        },
    }


def load_previous(domain_dir: Path) -> tuple[list[dict], dict[str, str], dict[str, str]]:
    """Return (entities, qid→id, name→id) from the previous entities.json.

    The QID map relies on the prior dataset embedding QIDs, which v1 didn't.
    Result: only the name index will match initially; QIDs become stable
    from v2 onwards.
    """
    path = domain_dir / "entities.json"
    if not path.exists():
        return [], {}, {}
    entities = json.loads(path.read_text(encoding="utf-8"))
    by_qid: dict[str, str] = {}
    by_name: dict[str, str] = {}
    for e in entities:
        by_name[e["name"]] = e["id"]
    return entities, by_qid, by_name


def diff_summary(prev: list[dict], curr: list[dict]) -> dict:
    prev_by_id = {e["id"]: e for e in prev}
    curr_by_id = {e["id"]: e for e in curr}
    added = sorted(set(curr_by_id) - set(prev_by_id))
    removed = sorted(set(prev_by_id) - set(curr_by_id))
    kept = sorted(set(curr_by_id) & set(prev_by_id))
    relined: list[dict] = []
    moved: list[dict] = []
    for id_ in kept:
        p = prev_by_id[id_]
        c = curr_by_id[id_]
        p_lines = p["attributes"]["lines"]["str_list"]
        c_lines = c["attributes"]["lines"]["str_list"]
        if p_lines != c_lines:
            relined.append({"id": id_, "name": c["name"], "before": p_lines, "after": c_lines})
        p_geo = p["attributes"]["geo"]["geo"]
        c_geo = c["attributes"]["geo"]["geo"]
        d_lat = abs(p_geo["lat"] - c_geo["lat"])
        d_lon = abs(p_geo["lon"] - c_geo["lon"])
        if d_lat > 0.01 or d_lon > 0.01:  # ~1.1 km
            moved.append({"id": id_, "name": c["name"], "delta": [round(d_lat, 4), round(d_lon, 4)]})
    return {
        "counts": {"before": len(prev), "after": len(curr)},
        "added": [{"id": i, "name": curr_by_id[i]["name"], "lines": curr_by_id[i]["attributes"]["lines"]["str_list"]} for i in added],
        "removed": [{"id": i, "name": prev_by_id[i]["name"], "lines": prev_by_id[i]["attributes"]["lines"]["str_list"]} for i in removed],
        "relined": relined,
        "moved": moved,
    }


def build_metadata(entities: list[dict], dataset_bytes: bytes, prior_version: str) -> dict:
    today = time.strftime("%Y-%m-%d")
    return {
        "id": "rer",
        "name": {
            "fr": "RER d'Île-de-France",
            "en": "Île-de-France RER",
        },
        "version": bump_minor(prior_version),
        "default_locale": "fr",
        "license": "ODbL-1.0",
        "sources": [
            "https://fr.wikipedia.org/w/api.php",
        ],
        "generator_constraints": {
            "min_candidates_per_cell": 2,
            "max_candidates_per_cell": 25,
            "max_predicate_family_repeats": 1,
            "row_predicate_count": 3,
            "col_predicate_count": 3,
        },
        "dataset_sha256": hashlib.sha256(dataset_bytes).hexdigest(),
        "entity_count": len(entities),
        "sources_versions": [
            {
                "id": "frwiki-category-rer-line",
                "url": "https://fr.wikipedia.org/wiki/Cat%C3%A9gorie:Ligne_A_du_RER_d%27%C3%8Ele-de-France",
                "date": today,
            },
            {
                "id": "frwiki-rest-summary",
                "url": "https://fr.wikipedia.org/api/rest_v1/page/summary/",
                "date": today,
            },
        ],
    }


def bump_minor(version: str) -> str:
    try:
        maj, minr, patch = version.split(".")
        return f"{maj}.{int(minr) + 1}.0"
    except ValueError:
        return "0.3.0"


# ---------- main ----------


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1] if __doc__ else "")
    parser.add_argument("--domain-root", type=Path, default=DOMAIN_DIR_DEFAULT)
    parser.add_argument("--apply", action="store_true", help="Write entities.json + metadata.json (default: dry-run).")
    args = parser.parse_args()

    domain_dir: Path = args.domain_root
    cache_dir = domain_dir / "tmp" / "frwiki"
    cache_dir.mkdir(parents=True, exist_ok=True)

    prev_entities, prev_by_qid, prev_by_name = load_previous(domain_dir)
    log(f"prior dataset: {len(prev_entities)} entities")

    per_line = collect_per_line(cache_dir)
    entities, wiki_titles = build_entities(per_line, prev_by_qid, prev_by_name, cache_dir)
    log(f"built {len(entities)} new entities, {len(wiki_titles)} frwiki titles tracked")

    diff = diff_summary(prev_entities, entities)
    log(
        f"diff: +{len(diff['added'])} added, -{len(diff['removed'])} removed, "
        f"~{len(diff['relined'])} relined, ~{len(diff['moved'])} moved"
    )

    prior_meta_path = domain_dir / "metadata.json"
    prior_version = "0.2.0"
    if prior_meta_path.exists():
        prior_version = json.loads(prior_meta_path.read_text(encoding="utf-8")).get("version", "0.2.0")

    entities_bytes = (json.dumps(entities, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    metadata = build_metadata(entities, entities_bytes, prior_version)

    if not args.apply:
        diff_path = domain_dir / "tmp" / "build_v2_diff.json"
        diff_path.write_text(json.dumps(diff, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        log(f"dry-run: wrote diff to {diff_path}")
        log(f"would bump version to {metadata['version']}")
        return 0

    (domain_dir / "entities.json").write_bytes(entities_bytes)
    meta_bytes = (json.dumps(metadata, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    (domain_dir / "metadata.json").write_bytes(meta_bytes)

    # Sidecar: { entity_id → frwiki article title }. Read by fame_score_rer.py
    # so it can skip its variant-guessing and resolve titles in O(1). Kept
    # next to entities.json (not in tmp/) because it's a build artifact the
    # downstream pipeline depends on; regenerated on every --apply.
    titles_bytes = (
        json.dumps(dict(sorted(wiki_titles.items())), ensure_ascii=False, indent=2) + "\n"
    ).encode("utf-8")
    (domain_dir / "wiki_titles.json").write_bytes(titles_bytes)
    log(
        f"wrote {len(entities)} entities + metadata version {metadata['version']}"
        f" + {len(wiki_titles)} wiki titles"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
