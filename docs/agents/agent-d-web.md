# Agent D — `web/`

## Périmètre

Tout `web/`.

## Livrables

### 1. Pages Astro

- `/` — accueil mobile-first, grille du jour, score, erreurs, partage.
- `/archives` — grilles passées (jouables sans leaderboard).
- `/leaderboard` — top du jour.
- `/profile` — pseudo, stats perso, déconnexion.
- `/legal/{privacy,cookies,terms}` — pages prerendues.
- `/duel/[id]` — page duel asynchrone (phase 2).

### 2. Composants Svelte 5

- `<Grid>` : 3×3 cases + en-têtes ligne/col + score + erreurs.
- `<CellButton>` : clic ouvre modal autocomplete.
- `<AutocompleteModal>` :
  - Input avec `normalize` (TS) + appel `/api/domains/{id}/autocomplete`.
  - Liste de suggestions (max 8), surlignage du match, navigation clavier.
  - Validation = `POST /api/games/{id}/play`.
- `<EndGameModal>` : score, partage emoji, copier le résultat, voir solutions.
- `<RulesModal>` : règles, switch "ne plus afficher".
- `<PredicateChip>` : libellé prédicat + `?` qui ouvre une modal explication.

### 3. État global

- Store Svelte minimal : `gameStore.ts` (id partie, plays, mistakes, score).
- Persistance localStorage : reprendre la partie en cours après refresh.

### 4. Client API typé

- `pnpm openapi:gen` produit `src/lib/api/types.ts` depuis `contracts/openapi.yaml`.
- Wrapper léger `src/lib/api/client.ts` qui type `fetch` via les types générés.

### 5. i18n (paraglide)

- `messages/{fr,en}.json`.
- Détection de langue : `Accept-Language` header → cookie `locale` → `lang` URL.

### 6. PWA

- `public/manifest.webmanifest` (déjà posé).
- Service worker avec workbox-precaching pour le shell + cache-first sur la grille du jour.

### 7. Accessibilité

- WCAG 2.1 AA.
- Tests Playwright + `axe-core` qui doivent retourner 0 violations.
- Combobox autocomplete avec `role="listbox"`, `aria-activedescendant`, navigation clavier complète.

### 8. Mobile-first

- 360 px de largeur cible. Tous les designs commencent là.
- Test obligatoire dans Playwright `chromium-mobile` (Pixel 5) + `webkit-mobile` (iPhone 13).
- Pas de hover-only interactions.

## Acceptance

- [ ] `pnpm dev` démarre, `/` rend la grille du jour.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm test` (vitest) passe.
- [ ] `pnpm exec playwright test` passe golden path + axe-core.
- [ ] Bundle JS final < 100 ko gzippé sur la page d'accueil.
