/**
 * paris-metro dataset ingestion pipeline.
 *
 * Sources :
 *  1. OSM Overpass API   — géo précise + dérivation des lignes via les relations route=subway.
 *  2. Mapping statique   — fallback de qualité (lignes connues) si Overpass ne livre rien
 *                          pour une station donnée. Voir lib/static-lines.ts.
 *  3. GTFS IDFM (option) — pas utilisé dans cette version : nécessite un token PRIM.
 *                          La structure du pipeline laisse la place à son ajout futur.
 *
 * Sorties :
 *  - domains/paris-metro/entities.json (commit, trié par id, schema-valide).
 *  - domains/paris-metro/metadata.json (bump version + dataset_sha256 + sources_versions).
 *
 * Cache :
 *  - tmp/osm-*.json (TTL 7j). Si frais, pas de réseau.
 *
 * Usage : pnpm --dir domains/paris-metro/scripts install && pnpm --dir domains/paris-metro/scripts ingest
 */

import { resolve, dirname } from "node:path";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { sha256Hex, writeJson } from "./lib/io.ts";
import { isInParisBbox, slugify } from "./lib/geo.ts";
import {
  loadOsmCaches,
  extractStations,
  indexStopNodes,
  buildNodeToLines,
  inferLinesForStation,
} from "./lib/osm.ts";
import { mergeStationsByName, type MergedStation } from "./lib/dedup.ts";
import { lookupStaticLines, compareLineRefs } from "./lib/static-lines.ts";
import { buildArrondPolygons, lookupArrondissement } from "./lib/arrondissement.ts";
import type { Entity } from "./lib/types.ts";

const VALID_LINES = new Set([
  "1", "2", "3", "3bis", "4", "5", "6", "7", "7bis",
  "8", "9", "10", "11", "12", "13", "14",
]);

const HERE = dirname(fileURLToPath(import.meta.url));
const DOMAIN_DIR = resolve(HERE, "..");
const TMP_DIR = resolve(DOMAIN_DIR, "tmp");
const ENTITIES_PATH = resolve(DOMAIN_DIR, "entities.json");
const METADATA_PATH = resolve(DOMAIN_DIR, "metadata.json");

const CACHE_PATHS = {
  stations: resolve(TMP_DIR, "osm-stations.json"),
  routes: resolve(TMP_DIR, "osm-routes.json"),
  stops: resolve(TMP_DIR, "osm-stops.json"),
  arrond: resolve(TMP_DIR, "osm-arrondissements.json"),
};

interface BuildSummary {
  total: number;
  inParis: number;
  withLinesFromOsm: number;
  withLinesFromStatic: number;
  withoutLines: string[];
}

async function main(): Promise<void> {
  log("paris-metro ingestion: starting");
  const caches = await loadCaches();
  const stations = extractAndMerge(caches);
  log(`OSM stations parsed: ${stations.length}`);

  const stopIndex = indexStopNodes(caches.stops);
  const nodeToLines = buildNodeToLines(caches.routes, VALID_LINES);
  const arrondPolys = buildArrondPolygons(caches.arrond);
  log(`OSM stops indexed: ${stopIndex.size}, route nodes mapped: ${nodeToLines.size}`);

  const entities = await buildEntities(stations, stopIndex, nodeToLines, arrondPolys);
  const sorted = entities.sort((a, b) => a.id.localeCompare(b.id));
  await writeEntities(sorted);
  await updateMetadata(sorted);
  reportSummary(sorted);
  log("paris-metro ingestion: done");
}

async function loadCaches() {
  log("loading OSM caches (Overpass, TTL 7d)");
  return loadOsmCaches(CACHE_PATHS);
}

function extractAndMerge(caches: Awaited<ReturnType<typeof loadCaches>>): MergedStation[] {
  const raw = extractStations(caches.stations);
  return mergeStationsByName(raw);
}

async function buildEntities(
  stations: MergedStation[],
  stopIndex: ReturnType<typeof indexStopNodes>,
  nodeToLines: ReturnType<typeof buildNodeToLines>,
  arrondPolys: ReturnType<typeof buildArrondPolygons>,
): Promise<Entity[]> {
  const usedIds = new Set<string>();
  const entities: Entity[] = [];
  for (const station of stations) {
    const entity = buildOneEntity(station, stopIndex, nodeToLines, arrondPolys, usedIds);
    if (entity) entities.push(entity);
  }
  return entities;
}

function buildOneEntity(
  s: MergedStation,
  stopIndex: ReturnType<typeof indexStopNodes>,
  nodeToLines: ReturnType<typeof buildNodeToLines>,
  arrondPolys: ReturnType<typeof buildArrondPolygons>,
  usedIds: Set<string>,
): Entity | null {
  const displayName = normalizeDisplayName(s.name);
  const id = uniqueId(displayName, usedIds);
  const lines = resolveLines({ ...s, name: displayName }, stopIndex, nodeToLines);
  if (lines.length === 0) return null;
  const point = { lat: s.lat, lon: s.lon };
  const arrond = lookupArrondissement(point, arrondPolys);
  const inParis = arrond > 0 || isInParisBbox(point);
  const entity: Entity = {
    id,
    name: displayName,
    attributes: {
      lines: { str_list: lines },
      geo: { geo: { lat: round6(point.lat), lon: round6(point.lon) } },
      in_paris: { bool: inParis },
      arrondissement: { num: arrond },
    },
  };
  if (s.aliases.length > 0) entity.aliases = s.aliases;
  return entity;
}

function uniqueId(name: string, used: Set<string>): string {
  const base = slugify(name);
  let candidate = base;
  let i = 2;
  while (used.has(candidate)) {
    candidate = `${base}-${i}`;
    i += 1;
  }
  used.add(candidate);
  return candidate;
}

/**
 * Stratégie hybride OSM ∪ static-lines.
 *
 * - Si la station est connue dans `static-lines.ts`, on FAIT confiance au statique
 *   (humain-vérifié). Cela résout les faux positifs de l'algo proximity sur des
 *   stations voisines (Mabillon ne doit pas hériter de la ligne 4 d'Odéon, etc.)
 *   ainsi que les faux positifs sur des gares non-métro (Pont Cardinet).
 * - Sinon on retombe sur l'inférence OSM via les relations route=subway, qui
 *   couvre les stations légitimes manquantes du mapping (extensions récentes).
 * - Si rien ne matche, la station sera dropée par le pipeline (lines.length === 0).
 */
function resolveLines(
  s: MergedStation,
  stopIndex: ReturnType<typeof indexStopNodes>,
  nodeToLines: ReturnType<typeof buildNodeToLines>,
): string[] {
  const fromStatic = lookupStaticLines(s.name);
  if (fromStatic && fromStatic.length > 0) return fromStatic;
  const fromOsm = inferLinesForStation({ lat: s.lat, lon: s.lon }, stopIndex, nodeToLines);
  return fromOsm;
}

/**
 * Normalise le nom canonique pour l'affichage : retire les suffixes parasites
 * type " (Métro)" qu'OSM ajoute parfois sur les stations homonymes (gares).
 */
function normalizeDisplayName(name: string): string {
  return name
    .replace(/\s*\((Métro|métro|Metro|metro)\)\s*$/u, "")
    .trim();
}

function round6(n: number): number {
  return Math.round(n * 1e6) / 1e6;
}

async function writeEntities(entities: Entity[]): Promise<void> {
  log(`writing ${entities.length} entities to ${ENTITIES_PATH}`);
  await writeJson(ENTITIES_PATH, entities);
}

async function updateMetadata(entities: Entity[]): Promise<void> {
  const raw = await readFile(METADATA_PATH, "utf8");
  const meta = JSON.parse(raw) as Record<string, unknown>;
  const datasetSha = sha256Hex(JSON.stringify(entities));
  const today = new Date().toISOString().slice(0, 10);
  meta["version"] = "0.2.0";
  meta["dataset_sha256"] = datasetSha;
  meta["entity_count"] = entities.length;
  meta["sources_versions"] = [
    { id: "osm-overpass-stations", url: "https://overpass-api.de/api/interpreter", date: today },
    { id: "osm-overpass-routes", url: "https://overpass-api.de/api/interpreter", date: today },
    { id: "static-lines-fallback", url: "see scripts/lib/static-lines.ts", date: today },
  ];
  await writeJson(METADATA_PATH, meta);
  log(`metadata.json bumped to ${meta["version"] as string}, dataset_sha256=${datasetSha.slice(0, 16)}`);
}

function reportSummary(entities: Entity[]): void {
  const summary: BuildSummary = {
    total: entities.length,
    inParis: entities.filter((e) => e.attributes.in_paris.bool).length,
    withLinesFromOsm: entities.filter((e) => e.attributes.lines.str_list.length > 0).length,
    withLinesFromStatic: 0,
    withoutLines: entities
      .filter((e) => e.attributes.lines.str_list.length === 0)
      .map((e) => e.name),
  };
  const lineDistribution: Record<string, number> = {};
  for (const e of entities) {
    for (const line of e.attributes.lines.str_list) {
      lineDistribution[line] = (lineDistribution[line] ?? 0) + 1;
    }
  }
  const ordered = Object.entries(lineDistribution).sort((a, b) =>
    compareLineRefs(a[0], b[0]),
  );
  log("--- summary ---");
  log(`total entities: ${summary.total}`);
  log(`in paris (intra-muros): ${summary.inParis}`);
  log(`stations per line: ${ordered.map(([k, v]) => `${k}=${v}`).join(", ")}`);
  if (summary.withoutLines.length > 0) {
    log(`stations without lines (dropped): ${summary.withoutLines.join(", ")}`);
  }
}

function log(msg: string): void {
  const ts = new Date().toISOString().slice(11, 19);
  process.stdout.write(`[${ts}] ${msg}\n`);
}

main().catch((err: unknown) => {
  process.stderr.write(`[ingest] fatal: ${err instanceof Error ? err.message : String(err)}\n`);
  process.exit(1);
});
