/**
 * Solo-mode helpers (parsing the `?seed=` query, building share URLs).
 *
 * The store and the API client already speak solo natively (PublicGrid has a
 * `seed` field, StartGameRequest accepts an optional `seed`). This module
 * stays a thin layer so the page-side glue can keep its hands off the store.
 */

/**
 * Read `?seed=<int>` from the current URL. Returns `undefined` outside the
 * browser, when the param is missing, or when it isn't a valid 32-bit int.
 */
export const parseSeedFromUrl = (): number | undefined => {
  if (typeof window === "undefined") return undefined;
  const raw = new URLSearchParams(window.location.search).get("seed");
  if (raw === null) return undefined;
  return parseSeed(raw);
};

/**
 * Parse a user-supplied seed string. We accept positive integers only,
 * capped at `2^32 - 1` to match the server-side `random_short_seed`.
 */
export const parseSeed = (raw: string): number | undefined => {
  const trimmed = raw.trim();
  if (!/^\d+$/.test(trimmed)) return undefined;
  const n = Number(trimmed);
  if (!Number.isFinite(n) || n < 0 || n > 0xffff_ffff) return undefined;
  return n;
};

/**
 * Build a shareable absolute URL for a solo grid, e.g.
 * `https://kalidoku.calixteair.fr/play?seed=314159`. Falls back to the relative
 * form when called outside the browser.
 */
export const buildSeedUrl = (seed: number, base?: string): string => {
  const path = `/play?seed=${seed}`;
  if (base) return `${base.replace(/\/+$/, "")}${path}`;
  if (typeof window === "undefined") return path;
  return `${window.location.origin}${path}`;
};
