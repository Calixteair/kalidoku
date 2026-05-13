import type { components } from "../api/types.js";

type PublicGrid = components["schemas"]["PublicGrid"];
type Cell = components["schemas"]["Cell"];

export interface CellAnswer {
  row: number;
  col: number;
  entityName: string;
  filledAt: string; // ISO
}

export interface PersistedGameState {
  gameId: string;
  gridId: string;
  domain: string;
  startedAt: string;
  playToken: string;
  playTokenExpiresAt: string;
  mistakesAllowed: number;
  mistakesLeft: number;
  score: number;
  answers: CellAnswer[];
  ended: boolean;
  finishedAt: string | null;
}

const isBrowser = (): boolean => typeof window !== "undefined";

/**
 * Per-domain localStorage key: `kalidoku.game.<domain>.v1`. Switching between
 * /paris-metro and /rer preserves both running games — no clobber on
 * navigation.
 *
 * The pre-multi-domain code used a single `kalidoku.game.v1` key; on first
 * load we migrate it into whichever domain its `state.domain` field points
 * at, then delete the legacy key. Deferred migration so users who never
 * played paris-metro don't get their key inflated with an empty entry.
 */
const LEGACY_KEY = "kalidoku.game.v1";
const storageKey = (domain: string): string => `kalidoku.game.${domain}.v1`;

const migrateLegacy = (): void => {
  if (!isBrowser()) return;
  const raw = window.localStorage.getItem(LEGACY_KEY);
  if (!raw) return;
  try {
    const legacy = JSON.parse(raw) as PersistedGameState;
    if (legacy && typeof legacy.domain === "string" && legacy.domain.length > 0) {
      const target = storageKey(legacy.domain);
      // Don't overwrite a fresh per-domain entry if one already exists.
      if (!window.localStorage.getItem(target)) {
        window.localStorage.setItem(target, raw);
      }
    }
  } catch {
    /* legacy payload corrupt — drop it */
  }
  window.localStorage.removeItem(LEGACY_KEY);
};

const loadPersisted = (domain: string): PersistedGameState | null => {
  if (!isBrowser()) return null;
  migrateLegacy();
  try {
    const raw = window.localStorage.getItem(storageKey(domain));
    if (!raw) return null;
    const parsed = JSON.parse(raw) as PersistedGameState;
    // Defensive: drop any entry whose `domain` field disagrees with its key,
    // e.g. if the user hand-edited storage or a future code shape diverged.
    if (parsed.domain !== domain) return null;
    return parsed;
  } catch {
    return null;
  }
};

const persist = (domain: string, state: PersistedGameState | null): void => {
  if (!isBrowser()) return;
  const key = storageKey(domain);
  if (state === null) {
    window.localStorage.removeItem(key);
    return;
  }
  window.localStorage.setItem(key, JSON.stringify(state));
};

const cellKey = (cell: Cell): string => `${cell.row},${cell.col}`;

/**
 * One store per (component × domain). Pass the active domain — Grid.svelte
 * already receives it as a prop, so it's a one-line change at the call site.
 */
export const createGameStore = (domain: string) => {
  const initial = loadPersisted(domain);

  let state = $state<PersistedGameState | null>(initial);
  let grid = $state<PublicGrid | null>(null);

  const answersByCell = $derived.by(() => {
    const map = new Map<string, CellAnswer>();
    if (!state) return map;
    for (const a of state.answers) {
      map.set(cellKey({ row: a.row, col: a.col }), a);
    }
    return map;
  });

  const isOver = $derived.by(() => {
    if (!state) return false;
    if (state.ended) return true;
    if (state.mistakesLeft <= 0) return true;
    return state.answers.length >= 9;
  });

  const startGame = (input: {
    gameId: string;
    gridId: string;
    domain: string;
    startedAt: string;
    playToken: string;
    playTokenExpiresAt: string;
    mistakesAllowed: number;
  }): void => {
    state = {
      ...input,
      mistakesLeft: input.mistakesAllowed,
      score: 0,
      answers: [],
      ended: false,
      finishedAt: null,
    };
    persist(domain, state);
  };

  const setGrid = (g: PublicGrid | null): void => {
    grid = g;
  };

  const recordPlay = (
    cell: Cell,
    entityName: string,
    result: { ok: boolean; scoreDelta: number; mistakesLeft: number; ended?: boolean | undefined },
  ): void => {
    if (!state) return;
    const next: PersistedGameState = {
      ...state,
      mistakesLeft: result.mistakesLeft,
      score: state.score + result.scoreDelta,
    };
    if (result.ok) {
      next.answers = [
        ...state.answers,
        { row: cell.row, col: cell.col, entityName, filledAt: new Date().toISOString() },
      ];
    }
    if (result.ended) {
      next.ended = true;
      next.finishedAt = new Date().toISOString();
    }
    state = next;
    persist(domain, state);
  };

  const endGame = (): void => {
    if (!state) return;
    state = { ...state, ended: true, finishedAt: new Date().toISOString() };
    persist(domain, state);
  };

  const clear = (): void => {
    state = null;
    grid = null;
    persist(domain, null);
  };

  return {
    get state(): PersistedGameState | null {
      return state;
    },
    get grid(): PublicGrid | null {
      return grid;
    },
    get answersByCell(): Map<string, CellAnswer> {
      return answersByCell;
    },
    get isOver(): boolean {
      return isOver;
    },
    startGame,
    setGrid,
    recordPlay,
    endGame,
    clear,
  };
};

export type GameStore = ReturnType<typeof createGameStore>;

export { cellKey };
