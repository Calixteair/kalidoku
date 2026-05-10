/**
 * Valide entities.json contre contracts/entity-schema.json.
 *
 * Vérifie en plus :
 *  - Unicité de id et name.
 *  - Lignes parmi {1-14, 3bis, 7bis}.
 *  - Coords dans la bbox Paris (incluant proche banlieue car certaines stations
 *    intra-muros ont leur centroïde OSM légèrement hors strict 1-20).
 *  - dataset_sha256 cohérent avec le contenu de entities.json.
 *
 * Exit 1 si violation.
 */

import { resolve, dirname } from "node:path";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import type { ValidateFunction } from "ajv";

import { sha256Hex } from "./lib/io.ts";
import type { Entity } from "./lib/types.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const DOMAIN_DIR = resolve(HERE, "..");
const REPO_ROOT = resolve(DOMAIN_DIR, "..", "..");

const ENTITIES_PATH = resolve(DOMAIN_DIR, "entities.json");
const METADATA_PATH = resolve(DOMAIN_DIR, "metadata.json");
const SCHEMA_PATH = resolve(REPO_ROOT, "contracts", "entity-schema.json");

const VALID_LINES = new Set([
  "1", "2", "3", "3bis", "4", "5", "6", "7", "7bis",
  "8", "9", "10", "11", "12", "13", "14",
]);

// Bbox élargie : la ligne 14 sud descend jusqu'à Aéroport d'Orly (lat ~48.726).
// On laisse une petite marge pour absorber les imprécisions OSM.
const PARIS_BBOX = {
  latMin: 48.72, latMax: 48.92,
  lonMin: 2.18,  lonMax: 2.51,
};

type DomainSchema = Record<string, unknown> & {
  $id?: string;
  $defs?: Record<string, unknown>;
};

async function main(): Promise<void> {
  const errors: string[] = [];
  const entities = await loadEntities();
  const schema = await loadSchema();

  // On charge le schema entier (pas seulement $defs.Entity) pour conserver les $ref
  // internes (#/$defs/AttributeValue, etc.) qu'Ajv résout via getSchema(uri#fragment).
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  ajv.addSchema(schema, "entity-schema");
  const validate = ajv.getSchema<Entity>("entity-schema#/$defs/Entity");
  if (!validate) {
    process.stderr.write("[validate] fatal: cannot resolve #/$defs/Entity from schema\n");
    process.exit(1);
  }

  for (const e of entities) {
    if (!(validate as ValidateFunction<Entity>)(e)) {
      const detail = ajv.errorsText((validate as ValidateFunction<Entity>).errors);
      errors.push(`schema(${e.id ?? "?"}): ${detail}`);
    }
  }

  errors.push(...checkUniqueness(entities, "id", (e) => e.id));
  errors.push(...checkUniqueness(entities, "name", (e) => e.name));
  errors.push(...checkLines(entities));
  errors.push(...checkGeo(entities));
  errors.push(...checkArrondissement(entities));
  errors.push(...(await checkMetadataHash(entities)));

  if (errors.length > 0) {
    for (const e of errors) process.stderr.write(`[validate] ${e}\n`);
    process.stderr.write(`[validate] FAIL: ${errors.length} issue(s)\n`);
    process.exit(1);
  }
  process.stdout.write(`[validate] OK: ${entities.length} entities, schema + invariants pass\n`);
}

async function loadEntities(): Promise<Entity[]> {
  const raw = await readFile(ENTITIES_PATH, "utf8");
  return JSON.parse(raw) as Entity[];
}

async function loadSchema(): Promise<DomainSchema> {
  const raw = await readFile(SCHEMA_PATH, "utf8");
  return JSON.parse(raw) as DomainSchema;
}

function checkUniqueness(
  entities: Entity[],
  label: string,
  keyFn: (e: Entity) => string,
): string[] {
  const seen = new Map<string, number>();
  for (const e of entities) {
    const k = keyFn(e);
    seen.set(k, (seen.get(k) ?? 0) + 1);
  }
  const dups = [...seen.entries()].filter(([, n]) => n > 1).map(([k]) => k);
  return dups.map((d) => `unique(${label}): duplicate value '${d}'`);
}

function checkLines(entities: Entity[]): string[] {
  const errors: string[] = [];
  for (const e of entities) {
    const list = e.attributes.lines.str_list;
    if (list.length === 0) {
      errors.push(`lines(${e.id}): empty list`);
      continue;
    }
    for (const line of list) {
      if (!VALID_LINES.has(line)) {
        errors.push(`lines(${e.id}): unknown line '${line}'`);
      }
    }
  }
  return errors;
}

function checkGeo(entities: Entity[]): string[] {
  const errors: string[] = [];
  for (const e of entities) {
    const { lat, lon } = e.attributes.geo.geo;
    if (lat < PARIS_BBOX.latMin || lat > PARIS_BBOX.latMax) {
      errors.push(`geo(${e.id}): lat ${lat} out of Paris bbox`);
    }
    if (lon < PARIS_BBOX.lonMin || lon > PARIS_BBOX.lonMax) {
      errors.push(`geo(${e.id}): lon ${lon} out of Paris bbox`);
    }
  }
  return errors;
}

function checkArrondissement(entities: Entity[]): string[] {
  const errors: string[] = [];
  for (const e of entities) {
    const a = e.attributes.arrondissement.num;
    if (!Number.isInteger(a) || a < 0 || a > 20) {
      errors.push(`arrondissement(${e.id}): invalid value ${a}`);
    }
    if (a > 0 && !e.attributes.in_paris.bool) {
      errors.push(`coherence(${e.id}): arrondissement ${a} but in_paris=false`);
    }
  }
  return errors;
}

async function checkMetadataHash(entities: Entity[]): Promise<string[]> {
  const meta = JSON.parse(await readFile(METADATA_PATH, "utf8")) as Record<string, unknown>;
  const expected = meta["dataset_sha256"];
  if (typeof expected !== "string") {
    return ["metadata: missing dataset_sha256"];
  }
  const actual = sha256Hex(JSON.stringify(entities));
  if (actual !== expected) {
    return [`metadata: dataset_sha256 mismatch (expected ${expected.slice(0, 16)}…, got ${actual.slice(0, 16)}…)`];
  }
  return [];
}

main().catch((err: unknown) => {
  process.stderr.write(`[validate] fatal: ${err instanceof Error ? err.message : String(err)}\n`);
  process.exit(1);
});
