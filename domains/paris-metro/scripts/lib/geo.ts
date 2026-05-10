/**
 * Helpers géo : haversine, slugify, détection arrondissement.
 */

const EARTH_RADIUS_M = 6_371_008.8;

export function haversineMeters(
  a: { lat: number; lon: number },
  b: { lat: number; lon: number },
): number {
  const toRad = (deg: number): number => (deg * Math.PI) / 180;
  const dLat = toRad(b.lat - a.lat);
  const dLon = toRad(b.lon - a.lon);
  const sinDLat = Math.sin(dLat / 2);
  const sinDLon = Math.sin(dLon / 2);
  const h =
    sinDLat * sinDLat +
    Math.cos(toRad(a.lat)) * Math.cos(toRad(b.lat)) * sinDLon * sinDLon;
  return 2 * EARTH_RADIUS_M * Math.asin(Math.sqrt(h));
}

const PARIS_BBOX = {
  latMin: 48.815,
  latMax: 48.902,
  lonMin: 2.224,
  lonMax: 2.470,
};

export function isInParisBbox(p: { lat: number; lon: number }): boolean {
  return (
    p.lat >= PARIS_BBOX.latMin &&
    p.lat <= PARIS_BBOX.latMax &&
    p.lon >= PARIS_BBOX.lonMin &&
    p.lon <= PARIS_BBOX.lonMax
  );
}

/**
 * Slugify FR : minuscules, accents enlevés, espaces et ponctuation -> tirets.
 * Doit produire un id stable et conforme au pattern ^[a-z0-9_][a-z0-9_-]{0,63}$.
 */
export function slugify(input: string): string {
  const ascii = input
    .normalize("NFD")
    .replace(/\p{Diacritic}+/gu, "")
    .replace(/Œ/g, "OE")
    .replace(/œ/g, "oe")
    .replace(/Æ/g, "AE")
    .replace(/æ/g, "ae")
    .replace(/ß/g, "ss");
  const lower = ascii.toLowerCase();
  const cleaned = lower
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/-{2,}/g, "-");
  return cleaned.length > 0 ? cleaned.slice(0, 64) : "x";
}
