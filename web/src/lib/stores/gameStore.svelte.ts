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

const STORAGE_KEY = "kalidoku.game.v1";

const isBrowser = (): boolean => typeof window !== "undefined";

const loadPersisted = (): PersistedGameState | null => {
  if (!isBrowser()) return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as PersistedGameState;
  } catch {
    return null;
  }
};

const persist = (state: PersistedGameState | null): void => {
  if (!isBrowser()) return;
  if (state === null) {
    window.localStorage.removeItem(STORAGE_KEY);
    return;
  }
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
};

const cellKey = (cell: Cell): string => `${cell.row},${cell.col}`;

export const createGameStore = () => {
  const initial = loadPersisted();

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
    persist(state);
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
    persist(state);
  };

  const endGame = (): void => {
    if (!state) return;
    state = { ...state, ended: true, finishedAt: new Date().toISOString() };
    persist(state);
  };

  const clear = (): void => {
    state = null;
    grid = null;
    persist(null);
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
