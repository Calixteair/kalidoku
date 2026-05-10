/**
 * paris-metro dataset ingestion. Owned by agent F.
 *
 * Combines:
 *  1. GTFS IDFM (https://prim.iledefrance-mobilites.fr) — canonical lines/stations/transfers
 *  2. OSM Overpass — geo precision, public_transport=stop_position centroids
 *  3. Wikidata SPARQL — opening date, named_after category, etymology
 *
 * Outputs:
 *  - domains/paris-metro/entities.json  (committed to repo, sorted by id)
 *
 * Usage:
 *   pnpm tsx domains/paris-metro/scripts/ingest.ts \
 *     --gtfs ./tmp/IDFM-gtfs.zip \
 *     --osm-cache ./tmp/osm-overpass.json \
 *     --wikidata-cache ./tmp/wikidata.json
 *
 * The script is idempotent and the output is reproducible (sources versioned by URL+hash).
 *
 * TODO(agent-F): see docs/agents/agent-f-domains.md for the step-by-step plan.
 */

console.error("agent-F: implement ingestion pipeline (see docs/agents/agent-f-domains.md)");
process.exit(1);
