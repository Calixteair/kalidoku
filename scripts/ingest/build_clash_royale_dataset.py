#!/usr/bin/env python3
"""
build_clash_royale_dataset.py — bootstrap `domains/clash-royale/entities.json`.

Sources
-------
- Supercell official API (https://api.clashroyale.com/v1/cards) for the
  canonical card list, including the iconUrls used by PR B for visuals.
  Requires a developer key — read from ~/.config/kalidoku/clash_royale_api_key
  (mode 600, *outside* the repo).
- RoyaleAPI community dump (https://royaleapi.github.io/cr-api-data/json/cards_stats.json)
  for combat stats: type, damage, hit_speed, range, speed, hitpoints,
  attacks_air, attacks_ground, flying_height, unlock_arena, etc. Supercell's
  /v1/cards is too sparse for kalidoku-style predicates so we join on name.

Output
------
- `domains/clash-royale/entities.json` — one entry per card with attributes
  the predicate families consume: rarity (str), elixir (num), type (str),
  attacks_air (bool), attacks_ground (bool), flies (bool), dps (num),
  hitpoints (num), speed (num), range (num), arena (num).
- `domains/clash-royale/metadata.json` — version 0.1.0, sources_versions
  records both Supercell + RoyaleAPI provenance.

Idempotency
-----------
Re-running the script overrides the dataset. fame_score is derived
deterministically from rarity (no Wikipedia pageviews here — pointless for
game cards).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import unicodedata
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Optional

SUPERCELL_ENDPOINT = "https://api.clashroyale.com/v1/cards"
ROYALEAPI_DUMP = "https://royaleapi.github.io/cr-api-data/json/cards_stats.json"
DEFAULT_KEY_PATH = Path.home() / ".config" / "kalidoku" / "clash_royale_api_key"
USER_AGENT = "kalidoku-cr-ingest/1.0 (https://github.com/Calixteair/kalidoku)"

# Rarity → fame_score. Inverted intuition: rare cards are *less* known
# casually so they grant *more* originality points. Matches the metro
# convention (fame_score 0 = niche → +100 contribution).
RARITY_FAME: dict[str, int] = {
    "common": 100,
    "rare": 80,
    "epic": 50,
    "legendary": 20,
    "champion": 5,
}

# Unlock-arena strings used by RoyaleAPI map onto numeric tier values.
# TrainingCamp = 0, Arena1..N = 1..N. The mapping is best-effort —
# unrecognised strings fall back to 0 so the arena predicate just sees
# "always unlocked" rather than failing.
ARENA_TIER_RE = re.compile(r"Arena(\d+)", re.IGNORECASE)


def log(msg: str) -> None:
    print(f"[cr-ingest] {msg}", file=sys.stderr, flush=True)


def normalise_name(name: str) -> str:
    """Lowercase, ASCII-only, strip non-alphanumerics. Used for the
    Supercell ↔ RoyaleAPI join: 'P.E.K.K.A' becomes 'pekka',
    'Mini P.E.K.K.A' becomes 'minipekka', etc."""
    s = unicodedata.normalize("NFKD", name)
    s = "".join(c for c in s if not unicodedata.combining(c))
    s = s.lower()
    s = re.sub(r"[^a-z0-9]+", "", s)
    return s


def slugify_id(name: str) -> str:
    """Entity id: lowercase ASCII + hyphen separators. Stays within the
    `[a-z0-9_-]{1,64}` constraint of entity-schema.json."""
    s = unicodedata.normalize("NFKD", name)
    s = "".join(c for c in s if not unicodedata.combining(c))
    s = s.lower()
    s = re.sub(r"[^a-z0-9]+", "-", s)
    s = s.strip("-")
    return s[:64] or "x"


def http_get_json(url: str, headers: Optional[dict] = None, retries: int = 3) -> Any:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
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
    raise RuntimeError(f"fetch failed: {url} → {last_err}")


# ---------- Supercell ----------


def fetch_supercell_cards(api_key: str) -> list[dict]:
    log("fetching canonical card list from Supercell API…")
    data = http_get_json(SUPERCELL_ENDPOINT, headers={"Authorization": f"Bearer {api_key}"})
    items = data.get("items", [])
    log(f"  → {len(items)} cards")
    return items


# ---------- RoyaleAPI ----------


def fetch_royaleapi_stats() -> dict[str, Any]:
    log("fetching combat stats from RoyaleAPI dump…")
    data = http_get_json(ROYALEAPI_DUMP)
    log(
        f"  → troops={len(data.get('troop', []))}, "
        f"buildings={len(data.get('building', []))}, "
        f"spells={len(data.get('spell', []))}, "
        f"characters={len(data.get('characters', []))}"
    )
    return data


def index_characters_by_name(characters: list[dict]) -> dict[str, dict]:
    """Lookup character entries (= the per-unit combat stats list) by their
    normalised name. Characters appear under `summon_character` references
    from troop/building entries."""
    out: dict[str, dict] = {}
    for c in characters:
        nm = c.get("name") or c.get("name_en")
        if not nm:
            continue
        out[normalise_name(nm)] = c
    return out


def index_kind_by_name(stats: dict, kind: str) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for entry in stats.get(kind, []):
        nm = entry.get("name") or entry.get("name_en")
        if not nm:
            continue
        out[normalise_name(nm)] = entry
    return out


# ---------- Combat-stats derivation ----------


def derive_combat_stats(name: str, kind_entry: Optional[dict], char_entry: Optional[dict]) -> dict[str, Any]:
    """Pick the most informative source. Troop combat stats live under
    `characters[summon_character]`; building stats are richer on the
    building entry itself; spell stats sit on the spell entry. We try
    the kind entry first, then fall back to characters."""
    source = char_entry or kind_entry or {}

    damage = source.get("damage") or 0
    hit_speed_ms = source.get("hit_speed") or 0
    dps = 0
    if isinstance(damage, (int, float)) and isinstance(hit_speed_ms, (int, float)) and hit_speed_ms > 0:
        dps = round((damage * 1000) / hit_speed_ms)

    return {
        "damage": int(damage) if damage is not None else 0,
        "hit_speed_ms": int(hit_speed_ms) if hit_speed_ms is not None else 0,
        "dps": int(dps),
        "hitpoints": int(source.get("hitpoints") or 0),
        "range": int(source.get("range") or 0),
        "speed": int(source.get("speed") or 0),
        "sight_range": int(source.get("sight_range") or 0),
        "attacks_air": bool(source.get("attacks_air")),
        "attacks_ground": bool(source.get("attacks_ground")),
        "flying_height": int(source.get("flying_height") or 0),
        "deploy_time_ms": int(source.get("deploy_time") or 0),
    }


def arena_tier(raw: Any) -> int:
    """Map 'TrainingCamp' / 'Arena1' / 'Arena12' to an integer 0..N. Returns
    0 for unrecognised strings — the predicate then treats the card as
    'always unlocked', better than failing the ingest."""
    if not raw:
        return 0
    s = str(raw)
    m = ARENA_TIER_RE.search(s)
    if m:
        return int(m.group(1))
    return 0


def download_icons(entities: list[dict], target_root: Path) -> int:
    """Download each entity's icon_url into <target_root>/<id>.png.
    Idempotent: skips when the file already exists with non-zero size.
    Returns the number of NEW files written. Failures are logged and
    don't abort — partial coverage is acceptable, CardIcon.svelte falls
    back gracefully on missing assets.

    Polite throttle: 100 ms between downloads. Supercell's api-assets CDN
    handles bursts but we don't push it."""
    target_root.mkdir(parents=True, exist_ok=True)
    written = 0
    skipped = 0
    failed: list[str] = []
    for ent in entities:
        url = ent.get("icon_url")
        if not url:
            continue
        path = target_root / f"{ent['id']}.png"
        if path.is_file() and path.stat().st_size > 0:
            skipped += 1
            continue
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=30) as r:
                payload = r.read()
            if not payload:
                failed.append(ent["id"])
                continue
            path.write_bytes(payload)
            written += 1
            time.sleep(0.1)
        except Exception as e:
            log(f"  WARN: icon download failed for {ent['id']}: {e}")
            failed.append(ent["id"])
    log(f"  icons: {written} downloaded, {skipped} already cached, {len(failed)} failed")
    if failed:
        log(f"  failed ids: {failed[:8]}{'...' if len(failed) > 8 else ''}")
    return written


# ---------- Build entities ----------


def build_entities(supercell_cards: list[dict], stats: dict) -> list[dict]:
    # Index every source by normalised name. Troops/buildings/spells +
    # characters: a card matches at least one of the lookup tables.
    by_troop = index_kind_by_name(stats, "troop")
    by_building = index_kind_by_name(stats, "building")
    by_spell = index_kind_by_name(stats, "spell")
    by_char = index_characters_by_name(stats.get("characters", []))

    out: list[dict] = []
    used_ids: set[str] = set()
    missing_stats: list[str] = []

    for card in supercell_cards:
        name = card.get("name")
        if not name:
            continue
        norm = normalise_name(name)

        # Determine type. Try troop → building → spell, in that order.
        kind_entry: Optional[dict] = None
        card_type = "Unknown"
        if norm in by_troop:
            kind_entry = by_troop[norm]
            card_type = "Troop"
        elif norm in by_building:
            kind_entry = by_building[norm]
            card_type = "Building"
        elif norm in by_spell:
            kind_entry = by_spell[norm]
            card_type = "Spell"

        # Characters table holds the unit combat stats for troops; for
        # buildings the kind_entry itself already carries them.
        summon = (kind_entry or {}).get("summon_character") if kind_entry else None
        char_entry = by_char.get(normalise_name(summon)) if summon else by_char.get(norm)

        combat = derive_combat_stats(name, kind_entry, char_entry)

        # Supercell-authoritative fields
        rarity = (card.get("rarity") or "").lower()
        elixir = card.get("elixirCost")
        icon_url = (card.get("iconUrls") or {}).get("medium", "")

        # Arena comes from troop/building/spell — characters don't carry it.
        unlock_arena_raw = (kind_entry or {}).get("unlock_arena")
        arena = arena_tier(unlock_arena_raw)

        if char_entry is None and kind_entry is None:
            missing_stats.append(name)

        ent_id = slugify_id(name)
        i = 1
        while ent_id in used_ids:
            ent_id = f"{slugify_id(name)}-{i}"
            i += 1
        used_ids.add(ent_id)

        # fame_score: rarity-driven, deterministic. fame_overrides.json can
        # still patch outliers afterwards (champion overshadowed by hype, etc).
        fame = RARITY_FAME.get(rarity, 50)

        entity = {
            "id": ent_id,
            "name": name,
            "attributes": {
                "rarity": {"str": rarity or "common"},
                "type": {"str": card_type},
                "elixir": {"num": float(elixir) if isinstance(elixir, (int, float)) else 0.0},
                "dps": {"num": float(combat["dps"])},
                "hitpoints": {"num": float(combat["hitpoints"])},
                "range": {"num": float(combat["range"])},
                "speed": {"num": float(combat["speed"])},
                "attacks_air": {"bool": combat["attacks_air"]},
                "attacks_ground": {"bool": combat["attacks_ground"]},
                "flies": {"bool": combat["flying_height"] > 0},
                "arena": {"num": float(arena)},
            },
            "fame_score": fame,
        }
        # Persist the Supercell icon URL on the entity. Schema (post-PR B) now
        # accepts it as an optional uri field; CardIcon.svelte reads
        # web/public/cards/<domain>/<id>.png at runtime, but the URL stays
        # available as a fallback / debugging anchor.
        if icon_url:
            entity["icon_url"] = icon_url
        out.append(entity)

    if missing_stats:
        log(f"  WARN: {len(missing_stats)} cards had no matching stats entry: {missing_stats[:8]}")
    log(f"  → {len(out)} entities ready")
    return sorted(out, key=lambda e: e["id"])


def build_metadata(entities: list[dict], dataset_bytes: bytes) -> dict:
    today = time.strftime("%Y-%m-%d")
    return {
        "id": "clash-royale",
        "name": {"fr": "Clash Royale", "en": "Clash Royale"},
        "version": "0.1.0",
        "default_locale": "fr",
        # Card data is published by Supercell + RoyaleAPI. Fan-content usage
        # under Supercell's Fan Content Policy — non-commercial editorial use.
        "license": "fan-content-policy",
        "sources": [
            "https://developer.clashroyale.com",
            "https://royaleapi.github.io/cr-api-data/",
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
            {"id": "supercell-cards", "url": "https://api.clashroyale.com/v1/cards", "date": today},
            {"id": "royaleapi-stats", "url": ROYALEAPI_DUMP, "date": today},
        ],
    }


def read_api_key(path: Path) -> str:
    if not path.is_file():
        raise SystemExit(
            f"missing API key at {path}\n"
            "place your Supercell developer JWT there with `chmod 600`."
        )
    key = path.read_text().strip()
    if not key:
        raise SystemExit(f"API key file {path} is empty")
    return key


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1] if __doc__ else "")
    parser.add_argument("--domain-root", type=Path, default=Path("domains/clash-royale"))
    parser.add_argument("--api-key-file", type=Path, default=DEFAULT_KEY_PATH)
    parser.add_argument(
        "--icons-dir",
        type=Path,
        default=Path("web/public/cards/clash-royale"),
        help="Where to write card .png files. Set to '' to skip the download pass.",
    )
    parser.add_argument(
        "--no-icons",
        action="store_true",
        help="Skip the icon download pass (data-only ingest).",
    )
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    api_key = read_api_key(args.api_key_file)
    supercell = fetch_supercell_cards(api_key)
    stats = fetch_royaleapi_stats()
    entities = build_entities(supercell, stats)
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

    if not args.no_icons:
        log(f"downloading card icons into {args.icons_dir}…")
        download_icons(entities, args.icons_dir)

    return 0


if __name__ == "__main__":
    sys.exit(main())
