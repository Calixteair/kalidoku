/**
 * Dédoublonne les RawStation OSM par nom canonique : on garde un centroïde des entrées
 * portant le même nom (cas fréquent des stations à plusieurs sorties).
 */

import type { RawStation } from "./types.ts";

export interface MergedStation {
  name: string;
  aliases: string[];
  lat: number;
  lon: number;
  osmIds: number[];
  wikidata?: string;
  wikipedia?: string;
}

export function mergeStationsByName(stations: RawStation[]): MergedStation[] {
  const groups = new Map<string, RawStation[]>();
  for (const s of stations) {
    const key = s.name;
    const list = groups.get(key);
    if (list) list.push(s);
    else groups.set(key, [s]);
  }
  const merged: MergedStation[] = [];
  for (const [name, group] of groups) {
    merged.push(mergeGroup(name, group));
  }
  return merged;
}

function mergeGroup(name: string, group: RawStation[]): MergedStation {
  const lat = average(group.map((s) => s.lat));
  const lon = average(group.map((s) => s.lon));
  const aliases = new Set<string>();
  let wikidata: string | undefined;
  let wikipedia: string | undefined;
  for (const s of group) {
    for (const a of s.aliases) aliases.add(a);
    if (!wikidata && s.wikidata) wikidata = s.wikidata;
    if (!wikipedia && s.wikipedia) wikipedia = s.wikipedia;
  }
  const result: MergedStation = {
    name,
    aliases: [...aliases].sort(),
    lat,
    lon,
    osmIds: group.map((s) => s.id).sort(),
  };
  if (wikidata) result.wikidata = wikidata;
  if (wikipedia) result.wikipedia = wikipedia;
  return result;
}

function average(values: number[]): number {
  return values.reduce((a, b) => a + b, 0) / values.length;
}
