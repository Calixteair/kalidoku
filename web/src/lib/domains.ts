/**
 * Source of truth for which domains are exposed by the frontend.
 *
 * Static-only — Astro's `output: 'static'` mode resolves `getStaticPaths`
 * at build time, so we can't query `/api/domains` here. When a new domain
 * lands in server/src/main.rs::active_domains we add a corresponding entry
 * below; CI catches the drift if the two get out of sync because the new
 * /[domain]/ pages would 404 in prod.
 *
 * The display labels are duplicated from the API just for the build-time
 * hub render; everywhere else (Grid header, LeaderboardView, etc.) the
 * label comes from paraglide messages keyed `grid_domain_<id>`.
 */

export interface DomainEntry {
  /** Stable id matching server-side active_domains. Lowercased, kebab-case. */
  id: string;
  /** French label used by the hub cards (Astro SSG can't call paraglide). */
  nameFr: string;
  /** English label used by the hub cards. */
  nameEn: string;
}

export const DOMAINS: readonly DomainEntry[] = [
  { id: "paris-metro", nameFr: "Métro de Paris", nameEn: "Paris Metro" },
  { id: "rer", nameFr: "RER d'Île-de-France", nameEn: "Île-de-France RER" },
  { id: "clash-royale", nameFr: "Clash Royale", nameEn: "Clash Royale" },
] as const;

/** Default domain for legacy redirects (`/play` → `/paris-metro/play`). */
export const DEFAULT_DOMAIN_ID = "paris-metro";

/** Astro getStaticPaths shape for /[domain]/* pages. */
export const domainStaticPaths = (): { params: { domain: string } }[] =>
  DOMAINS.map((d) => ({ params: { domain: d.id } }));

/** Lookup helper used by the [domain] pages to localise the picker chip. */
export const findDomain = (id: string): DomainEntry | undefined => DOMAINS.find((d) => d.id === id);
