#!/usr/bin/env python3
"""
build_rer_dataset.py — bootstrap `domains/rer/entities.json` from OSM.

Strategy
--------
WDQS is rate-limited too aggressively (1 req/min since the 2024 outage) to
fit a fresh ingest cycle. Pivot on OSM Overpass which has the same data
plus a richer station↔line mapping via the `route=train|light_rail` /
`network=RER` relations.

1. One Overpass query returns every `relation[type=route][network=RER]`
   plus its node members. The `ref` tag carries the trunk letter A..E.
2. For each station node we keep its name, geo, and the set of RER lines
   it appears on (union across relations).
3. `zone tarifaire` is not present on OSM nodes — left at 0; the
   downstream predicates can still operate on `lines`, `geo`, `in_paris`
   (derived heuristically from a Paris bbox), and name-based families.

Output
------
- `domains/rer/entities.json` (sorted by id)
- `domains/rer/metadata.json` (version 0.1.0, sources_versions, sha256)
- The `tmp/` directory under the domain is gitignored, so any audit
  artefact left here won't leak into the PR.

Idempotency
-----------
Re-running the script overwrites the dataset.
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

OVERPASS_ENDPOINT = "https://overpass-api.de/api/interpreter"
USER_AGENT = (
    "kalidoku-rer-ingest/1.0 "
    "(https://github.com/Calixteair/kalidoku contact: reymond.calixte@gmail.com)"
)

# Pull every RER route relation in Île-de-France + their stop-role member
# nodes only. Avoiding `.routes >;` because it pulls every member node
# (including `stop_entry_only`, `platform`, `node` markers between stops)
# — `node(r:"stop")` keeps only the canonical station nodes.
OVERPASS_QUERY = """
[out:json][timeout:120];
area["name"="Île-de-France"]->.idf;
relation["type"="route"]["network"="RER"](area.idf)->.routes;
.routes out body;
node(r.routes:"stop");
out body;
"""

# Paris bbox (inner ring road). Used to set `in_paris=true` on stations
# whose OSM node sits inside. Not as crisp as fare zones, but adequate for
# the `Hors Paris intra-muros` predicate family.
PARIS_BBOX = (48.815, 2.224, 48.902, 2.470)


def log(msg: str) -> None:
    print(f"[rer-ingest] {msg}", file=sys.stderr, flush=True)


def normalise_id(name: str) -> str:
    """Entity id: lowercase ascii + `-` separators, matches the
    `^[a-z0-9_][a-z0-9_-]{0,63}$` constraint of entity-schema.json."""
    s = unicodedata.normalize("NFKD", name)
    s = "".join(c for c in s if not unicodedata.combining(c))
    s = s.lower()
    s = re.sub(r"[^a-z0-9]+", "-", s)
    s = s.strip("-")
    return s[:64] or "x"


def http_get_json(url: str, retries: int = 3) -> dict:
    req = urllib.request.Request(url)
    req.add_header("User-Agent", USER_AGENT)
    req.add_header("Accept", "application/json")
    last_err: Optional[Exception] = None
    for attempt in range(retries):
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                return json.loads(r.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt < retries - 1:
                wait = 2 ** attempt
                log(f"  HTTP {e.code}, retry in {wait}s")
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
    raise RuntimeError(f"fetch failed: {last_err}")


def fetch_overpass() -> dict:
    log("fetching RER routes + member nodes from OSM Overpass…")
    url = f"{OVERPASS_ENDPOINT}?data={urllib.parse.quote(OVERPASS_QUERY)}"
    return http_get_json(url)


def in_paris(lat: float, lon: float) -> bool:
    return PARIS_BBOX[0] <= lat <= PARIS_BBOX[2] and PARIS_BBOX[1] <= lon <= PARIS_BBOX[3]


# Strip OSM platform/track suffixes so two physically separate nodes that
# share a station collapse into one entity. Matches "Châtelet - Voie 1",
# "Brétigny - Voie 6", "Étampes - Quai 3", "Choisy-le-Roi - Voie 1B", etc.
_SUFFIX_RE = re.compile(
    r"\s*[-–—]\s*(?:Voie|Quai|Platform)\s+\S+\s*$",
    re.IGNORECASE,
)


def canonical_station_name(raw: str) -> str:
    return _SUFFIX_RE.sub("", raw).strip()


def build_entities(overpass: dict) -> list[dict]:
    elements = overpass.get("elements", [])
    nodes = {e["id"]: e for e in elements if e["type"] == "node"}
    relations = [e for e in elements if e["type"] == "relation"]

    # node_id → set of RER trunk letters from every relation it sits in.
    node_lines: dict[int, set[str]] = {}
    for rel in relations:
        tags = rel.get("tags", {})
        ref = tags.get("ref", "")
        # Relations sometimes carry numbered refs like "A1", "B4" — collapse
        # the trunk letter so a station counts as "served by RER A".
        m = re.match(r"^([A-E])", ref)
        if not m:
            continue
        letter = m.group(1)
        for member in rel.get("members", []):
            if member.get("type") != "node":
                continue
            node_lines.setdefault(member["ref"], set()).add(letter)

    # First pass: group by canonical (suffix-stripped) name. For each group,
    # union the lines across every physical node and keep the geo of the
    # node closest to the group centroid. Skipping nodes without a name
    # and nodes that aren't referenced by any RER route relation.
    groups: dict[str, dict] = {}
    skipped_no_lines = 0
    skipped_no_name = 0
    for node_id, n in nodes.items():
        if node_id not in node_lines:
            skipped_no_lines += 1
            continue
        tags = n.get("tags", {})
        raw_name = tags.get("name") or tags.get("name:fr") or ""
        if not raw_name:
            skipped_no_name += 1
            continue
        lat, lon = n.get("lat"), n.get("lon")
        if lat is None or lon is None:
            continue
        canonical = canonical_station_name(raw_name)
        if not canonical:
            continue
        g = groups.setdefault(
            canonical,
            {
                "name": canonical,
                "lines": set(),
                "lats": [],
                "lons": [],
            },
        )
        g["lines"].update(node_lines[node_id])
        g["lats"].append(float(lat))
        g["lons"].append(float(lon))

    out = []
    used_ids: set[str] = set()
    skipped_no_lines_after_group = 0
    for canonical, g in groups.items():
        if not g["lines"]:
            skipped_no_lines_after_group += 1
            continue
        lat = sum(g["lats"]) / len(g["lats"])
        lon = sum(g["lons"]) / len(g["lons"])
        base = normalise_id(canonical)
        ent_id = base
        i = 1
        while ent_id in used_ids:
            ent_id = f"{base}-{i}"
            i += 1
        used_ids.add(ent_id)
        out.append(
            {
                "id": ent_id,
                "name": canonical,
                "attributes": {
                    "lines": {"str_list": sorted(g["lines"])},
                    "geo": {"geo": {"lat": lat, "lon": lon}},
                    "in_paris": {"bool": in_paris(lat, lon)},
                    # Wikidata's P5031 (zone tarifaire) is sparsely populated
                    # and OSM doesn't expose it. Left at 0; zone-based
                    # predicates require a follow-up enrich pass.
                    "zone": {"num": 0.0},
                },
            }
        )
    log(
        f"  → {len(out)} stations after canonical dedup "
        f"(skipped: {skipped_no_lines} no-lines, {skipped_no_name} no-name)"
    )
    return sorted(out, key=lambda e: e["id"])


def build_metadata(entities: list[dict], dataset_bytes: bytes) -> dict:
    today = time.strftime("%Y-%m-%d")
    return {
        "id": "rer",
        "name": {
            "fr": "RER d'Île-de-France",
            "en": "Île-de-France RER",
        },
        "version": "0.1.0",
        "default_locale": "fr",
        "license": "ODbL-1.0",
        "sources": [
            "https://overpass-api.de/api/interpreter",
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
                "id": "osm-overpass-rer-routes",
                "url": "https://overpass-api.de/api/interpreter",
                "date": today,
            },
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1] if __doc__ else "")
    parser.add_argument("--domain-root", type=Path, default=Path("domains/rer"))
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    overpass = fetch_overpass()
    entities = build_entities(overpass)
    if not entities:
        log("no entities produced — aborting")
        return 2

    entities_bytes = (json.dumps(entities, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    metadata = build_metadata(entities, entities_bytes)

    if args.dry_run:
        log("dry-run: skipping writes")
        log(f"would write {len(entities)} entities, metadata version {metadata['version']}")
        return 0

    args.domain_root.mkdir(parents=True, exist_ok=True)
    (args.domain_root / "entities.json").write_bytes(entities_bytes)
    log(f"wrote {args.domain_root / 'entities.json'}")

    meta_bytes = (json.dumps(metadata, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    (args.domain_root / "metadata.json").write_bytes(meta_bytes)
    log(f"wrote {args.domain_root / 'metadata.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
