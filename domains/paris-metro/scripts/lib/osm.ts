/**
 * Extraction depuis les caches Overpass : stations, mapping nodes-stops -> lignes.
 */

import { readJsonIfFresh, fetchJson, writeJson } from "./io.ts";
import { compareLineRefs } from "./static-lines.ts";
import { haversineMeters } from "./geo.ts";
import type { OverpassResponse, OverpassNode, RawStation } from "./types.ts";

const OVERPASS_URL = "https://overpass-api.de/api/interpreter";
const CACHE_DAYS = 7;

const BBOX = "48.795,2.180,48.920,2.510";

const STATIONS_QUERY = `[out:json][timeout:120];(node(${BBOX})[railway=station][station=subway];);out body;`;
const ROUTES_QUERY = `[out:json][timeout:180];(relation(${BBOX})[type=route][route=subway];);out body;`;
const STOPS_QUERY = `[out:json][timeout:180];relation(${BBOX})[type=route][route=subway];node(r:"stop")->.s;node(r:"stop_entry_only")->.se;node(r:"stop_exit_only")->.sx;(.s; .se; .sx;);out;`;
const ARROND_QUERY = `[out:json][timeout:90];relation["admin_level"="9"]["boundary"="administrative"](48.815,2.224,48.902,2.470);out body;way(r);out geom;`;

export interface OsmCaches {
  stations: OverpassResponse;
  routes: OverpassResponse;
  stops: OverpassResponse;
  arrond: OverpassResponse;
}

export interface OsmCachePaths {
  stations: string;
  routes: string;
  stops: string;
  arrond: string;
}

export async function loadOsmCaches(paths: OsmCachePaths): Promise<OsmCaches> {
  const stations = await readOrFetch(paths.stations, STATIONS_QUERY);
  const routes = await readOrFetch(paths.routes, ROUTES_QUERY);
  const stops = await readOrFetch(paths.stops, STOPS_QUERY);
  const arrond = await readOrFetch(paths.arrond, ARROND_QUERY);
  return { stations, routes, stops, arrond };
}

async function readOrFetch(path: string, query: string): Promise<OverpassResponse> {
  const cached = await readJsonIfFresh<OverpassResponse>(path, CACHE_DAYS);
  if (cached) return cached;
  const fresh = await fetchJson<OverpassResponse>(OVERPASS_URL, `data=${encodeURIComponent(query)}`);
  await writeJson(path, fresh);
  return fresh;
}

/**
 * Extrait la liste plate des stations métro Paris depuis la réponse Overpass.
 */
export function extractStations(resp: OverpassResponse): RawStation[] {
  const out: RawStation[] = [];
  for (const el of resp.elements) {
    if (el.type !== "node") continue;
    const tags = el.tags ?? {};
    if (tags["station"] !== "subway") continue;
    const name = tags["name"]?.trim();
    if (!name) continue;
    out.push({
      id: el.id,
      name,
      lat: el.lat,
      lon: el.lon,
      aliases: collectAliases(tags),
      ...(tags["wikidata"] ? { wikidata: tags["wikidata"] } : {}),
      ...(tags["wikipedia"] ? { wikipedia: tags["wikipedia"] } : {}),
    });
  }
  return out;
}

function collectAliases(tags: Record<string, string>): string[] {
  const set = new Set<string>();
  if (tags["old_name"]) set.add(tags["old_name"]);
  if (tags["alt_name"]) set.add(tags["alt_name"]);
  if (tags["short_name"]) set.add(tags["short_name"]);
  if (tags["loc_name"]) set.add(tags["loc_name"]);
  if (tags["official_name"]) set.add(tags["official_name"]);
  return [...set].filter((s) => s && s !== tags["name"]);
}

/**
 * Indexe chaque node-stop OSM (id) -> coords, depuis le cache `stops`.
 */
export function indexStopNodes(stops: OverpassResponse): Map<number, OverpassNode> {
  const m = new Map<number, OverpassNode>();
  for (const el of stops.elements) {
    if (el.type === "node") m.set(el.id, el);
  }
  return m;
}

/**
 * Construit la mapping nodeId -> set(line_ref) via les relations subway.
 */
export function buildNodeToLines(
  routes: OverpassResponse,
  validLines: Set<string>,
): Map<number, Set<string>> {
  const result = new Map<number, Set<string>>();
  for (const el of routes.elements) {
    if (el.type !== "relation") continue;
    const ref = el.tags?.["ref"];
    if (!ref || !validLines.has(ref)) continue;
    for (const member of el.members ?? []) {
      if (member.type !== "node") continue;
      if (!member.role.startsWith("stop")) continue;
      let lines = result.get(member.ref);
      if (!lines) {
        lines = new Set<string>();
        result.set(member.ref, lines);
      }
      lines.add(ref);
    }
  }
  return result;
}

/**
 * Pour une station donnée, trouve les lignes via les nodes-stops à <maxMeters.
 */
export function inferLinesForStation(
  station: { lat: number; lon: number },
  stopIndex: Map<number, OverpassNode>,
  nodeToLines: Map<number, Set<string>>,
  maxMeters = 200,
): string[] {
  const lines = new Set<string>();
  for (const [nodeId, node] of stopIndex) {
    const dist = haversineMeters(station, node);
    if (dist > maxMeters) continue;
    const ls = nodeToLines.get(nodeId);
    if (!ls) continue;
    for (const l of ls) lines.add(l);
  }
  return [...lines].sort(compareLineRefs);
}
