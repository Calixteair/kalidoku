/**
 * Types partagés pour le pipeline paris-metro.
 *
 * Restent fidèles au schema contracts/entity-schema.json.
 */

export type AttributeValue =
  | { str: string }
  | { num: number }
  | { bool: boolean }
  | { str_list: string[] }
  | { geo: { lat: number; lon: number } };

export type EntityAttributes = {
  lines: { str_list: string[] };
  geo: { geo: { lat: number; lon: number } };
  in_paris: { bool: boolean };
  arrondissement: { num: number };
  [key: string]: AttributeValue;
};

export interface Entity {
  id: string;
  name: string;
  aliases?: string[];
  attributes: EntityAttributes;
}

export interface OverpassNode {
  type: "node";
  id: number;
  lat: number;
  lon: number;
  tags?: Record<string, string>;
}

export interface OverpassRelationMember {
  type: "node" | "way" | "relation";
  ref: number;
  role: string;
}

export interface OverpassRelation {
  type: "relation";
  id: number;
  tags?: Record<string, string>;
  members?: OverpassRelationMember[];
}

export type OverpassElement = OverpassNode | OverpassRelation;

export interface OverpassResponse {
  version?: number;
  generator?: string;
  elements: OverpassElement[];
}

export interface RawStation {
  id: number;
  name: string;
  lat: number;
  lon: number;
  aliases: string[];
  wikidata?: string;
  wikipedia?: string;
}
