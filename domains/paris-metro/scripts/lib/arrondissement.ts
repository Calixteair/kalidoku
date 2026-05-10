/**
 * Détermine l'arrondissement parisien d'une station depuis sa lat/lon.
 *
 * Stratégie : ray-casting point-in-polygon contre les contours OSM des arrondissements.
 * Les contours viennent d'une requête Overpass `relation[admin_level=9]` cachée localement.
 */

import type { OverpassRelation } from "./types.ts";

interface OverpassWay {
  type: "way";
  id: number;
  geometry?: { lat: number; lon: number }[];
  nodes?: number[];
}

type ArrondElement = OverpassRelation | OverpassWay;

interface OverpassArrondResponse {
  elements: ArrondElement[];
}

interface ArrondPolygon {
  insee: number;
  name: string;
  rings: { lat: number; lon: number }[][];
}

/**
 * Construit la liste des polygones d'arrondissement depuis la réponse Overpass.
 * Chaque relation est composée de ways `outer` qu'on assemble en anneaux fermés.
 */
export function buildArrondPolygons(resp: OverpassArrondResponse): ArrondPolygon[] {
  const ways = new Map<number, OverpassWay>();
  for (const el of resp.elements) {
    if (el.type === "way") ways.set(el.id, el);
  }
  const polygons: ArrondPolygon[] = [];
  for (const el of resp.elements) {
    if (el.type !== "relation") continue;
    const insee = parseInseeCode(el.tags?.["ref:INSEE"]);
    if (insee === null) continue;
    const rings = assembleRings(el, ways);
    if (rings.length === 0) continue;
    polygons.push({ insee, name: el.tags?.name ?? "", rings });
  }
  return polygons;
}

function parseInseeCode(raw: string | undefined): number | null {
  if (!raw) return null;
  const m = /^751(\d{2})$/.exec(raw);
  if (!m) return null;
  const n = Number.parseInt(m[1]!, 10);
  return n >= 1 && n <= 20 ? n : null;
}

function assembleRings(
  rel: OverpassRelation,
  ways: Map<number, OverpassWay>,
): { lat: number; lon: number }[][] {
  const segments: { lat: number; lon: number }[][] = [];
  for (const m of rel.members ?? []) {
    if (m.type !== "way" || m.role !== "outer") continue;
    const w = ways.get(m.ref);
    if (!w?.geometry || w.geometry.length < 2) continue;
    segments.push([...w.geometry]);
  }
  return stitchSegments(segments);
}

/**
 * Recolle des segments tête-bêche en anneaux fermés (gestion de l'ordre).
 */
function stitchSegments(
  segments: { lat: number; lon: number }[][],
): { lat: number; lon: number }[][] {
  const remaining = segments.map((s) => [...s]);
  const rings: { lat: number; lon: number }[][] = [];
  while (remaining.length > 0) {
    const ring = remaining.shift()!;
    let extended = true;
    while (extended) {
      extended = false;
      for (let i = 0; i < remaining.length; i += 1) {
        const seg = remaining[i]!;
        if (sameCoord(ring[ring.length - 1]!, seg[0]!)) {
          ring.push(...seg.slice(1));
          remaining.splice(i, 1);
          extended = true;
          break;
        }
        if (sameCoord(ring[ring.length - 1]!, seg[seg.length - 1]!)) {
          ring.push(...seg.slice(0, -1).reverse());
          remaining.splice(i, 1);
          extended = true;
          break;
        }
      }
    }
    if (ring.length >= 4 && sameCoord(ring[0]!, ring[ring.length - 1]!)) {
      rings.push(ring);
    }
  }
  return rings;
}

function sameCoord(
  a: { lat: number; lon: number },
  b: { lat: number; lon: number },
): boolean {
  return Math.abs(a.lat - b.lat) < 1e-7 && Math.abs(a.lon - b.lon) < 1e-7;
}

/**
 * Ray casting : retourne true si `p` est à l'intérieur d'un anneau.
 */
function pointInRing(p: { lat: number; lon: number }, ring: { lat: number; lon: number }[]): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i, i += 1) {
    const a = ring[i]!;
    const b = ring[j]!;
    const intersect =
      a.lat > p.lat !== b.lat > p.lat &&
      p.lon < ((b.lon - a.lon) * (p.lat - a.lat)) / (b.lat - a.lat) + a.lon;
    if (intersect) inside = !inside;
  }
  return inside;
}

/**
 * Cherche l'arrondissement contenant le point. Retourne 0 si hors Paris.
 */
export function lookupArrondissement(
  point: { lat: number; lon: number },
  polygons: ArrondPolygon[],
): number {
  for (const poly of polygons) {
    for (const ring of poly.rings) {
      if (pointInRing(point, ring)) return poly.insee;
    }
  }
  return 0;
}
