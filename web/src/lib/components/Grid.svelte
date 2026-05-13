<script lang="ts">
  import BookOpen from "lucide-svelte/icons/book-open";
  import Flag from "lucide-svelte/icons/flag";
  import Play from "lucide-svelte/icons/play";
  import * as m from "../../paraglide/messages.js";
  import { api, ApiError } from "../api/client.js";
  import type { components } from "../api/types.js";
  import AutocompleteModal from "./AutocompleteModal.svelte";
  import CellButton from "./CellButton.svelte";
  import EndGameModal from "./EndGameModal.svelte";
  import PredicateChip from "./PredicateChip.svelte";
  import RulesModal from "./RulesModal.svelte";
  import { createGameStore, cellKey } from "../stores/gameStore.svelte.js";
  import { parseSeedFromUrl } from "../solo.js";

  type PublicGrid = components["schemas"]["PublicGrid"];
  type Cell = components["schemas"]["Cell"];
  type EndGameView = components["schemas"]["EndGameView"];

  interface Props {
    domain: string;
    mode?: "daily" | "solo" | "duel";
    /** Solo mode only: regenerate this seed instead of picking a fresh one. */
    seed?: number | undefined;
    /** Duel mode only: id of the pinned grid the duel reuses. */
    duelGridId?: string | undefined;
  }

  let { domain, mode = "daily", seed, duelGridId }: Props = $props();

  // Solo only: when no seed was provided as a prop, see if the URL pinned one
  // (share link `/play?seed=42`). Lets a player resume a friend's grid by
  // visiting the share URL.
  const effectiveSeed = $derived<number | undefined>(
    mode === "solo" && seed === undefined ? parseSeedFromUrl() : seed,
  );

  const store = createGameStore();

  let gridLoading = $state(true);
  let gridError = $state<string | null>(null);
  let starting = $state(false);
  let startError = $state<string | null>(null);
  let playError = $state<string | null>(null);

  let selectedCell = $state<Cell | null>(null);
  let rulesOpen = $state(false);
  let endGameOpen = $state(false);
  let endGameView = $state<EndGameView | null>(null);
  // Cells that just took a wrong answer — used to trigger a brief shake/flash
  // so the player gets immediate feedback even though the cell stays empty.
  let wrongCells = $state<Set<string>>(new Set());

  const flashWrong = (cell: Cell): void => {
    const key = cellKey(cell);
    wrongCells = new Set([...wrongCells, key]);
    setTimeout(() => {
      wrongCells = new Set([...wrongCells].filter((k) => k !== key));
    }, 700);
  };

  const currentLocale = (): "fr" | "en" => {
    if (typeof document === "undefined") return "fr";
    const lang = document.documentElement.lang;
    return lang === "en" ? "en" : "fr";
  };

  const loadGrid = async (): Promise<void> => {
    // Duel: the consent step happened upstream on /duel (the friend clicked
    // "Play this grid"). By the time Grid mounts with mode=duel + duelGridId,
    // we should immediately start the game so cells render. Mirror solo's
    // logic but pinned to the duel's grid_id rather than a fresh one: if the
    // persisted session points at the same grid, resume; otherwise clear and
    // start anew.
    if (mode === "duel") {
      if (duelGridId && store.state && store.state.gridId !== duelGridId) {
        store.clear();
      }
      const hasActiveDuel =
        store.state !== null &&
        !store.state.ended &&
        store.grid !== null &&
        store.state.gridId === duelGridId;
      if (!hasActiveDuel) {
        await startGame();
      }
      gridLoading = false;
      return;
    }
    // Solo: a fresh /play visit must already see a grid. We auto-start a game
    // unless there is a persisted, unfinished solo session whose grid is
    // already in the store (e.g. user refreshed mid-game). The startGame call
    // both pins the grid and seeds the store.
    if (mode === "solo") {
      const hasActiveSolo = store.state !== null && !store.state.ended && store.grid !== null;
      if (!hasActiveSolo) {
        await startGame();
      }
      gridLoading = false;
      return;
    }
    gridLoading = true;
    gridError = null;
    try {
      const g = await api.get("/api/grids/{domain}/today", { domain }, { locale: currentLocale() });
      // Drop a persisted game pointing at a different grid_id — that's a
      // session from yesterday (or another domain) whose answers would
      // otherwise render on today's grid as if they were valid. Without this
      // the player walks back in and sees yesterday's cells locked on
      // today's puzzle, unable to play.
      if (store.state && store.state.gridId !== g.id) {
        store.clear();
      }
      store.setGrid(g);
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        gridError = m.error_grid_unavailable();
      } else {
        gridError = m.error_network();
      }
    } finally {
      gridLoading = false;
    }
  };

  const startGame = async (): Promise<void> => {
    starting = true;
    startError = null;
    try {
      const data = await api.post("/api/games", undefined, {
        domain,
        mode,
        ...(mode === "solo" && typeof effectiveSeed === "number" ? { seed: effectiveSeed } : {}),
        ...(mode === "duel" && duelGridId ? { duelGridId } : {}),
      });
      store.startGame({
        gameId: data.game.id,
        gridId: data.grid.id,
        domain,
        startedAt: data.game.startedAt,
        playToken: data.playToken.token,
        playTokenExpiresAt: data.playToken.expiresAt,
        mistakesAllowed: data.grid.mistakesAllowed,
      });
      store.setGrid(data.grid);
    } catch (err) {
      if (err instanceof ApiError) {
        startError = err.message || m.error_game_start();
      } else {
        startError = m.error_game_start();
      }
    } finally {
      starting = false;
    }
  };

  const onCellSelect = (row: number, col: number): void => {
    if (store.isOver) return;
    if (!store.state) {
      void startGame().then(() => {
        if (store.state) selectedCell = { row, col };
      });
      return;
    }
    selectedCell = { row, col };
  };

  const closeAutocomplete = (): void => {
    selectedCell = null;
  };

  const submitAnswer = async (entity: { id: string; name: string }): Promise<void> => {
    const cell = selectedCell;
    const state = store.state;
    if (!cell || !state) return;
    playError = null;
    try {
      const res = await api.post(
        "/api/games/{gameId}/play",
        { gameId: state.gameId },
        { cell, answer: entity.name, playToken: state.playToken },
      );
      store.recordPlay(cell, entity.name, res);
      if (!res.ok) flashWrong(cell);
      // Auto-open the end-of-game modal once the server signals it (3 mistakes
      // or 9 cells solved). Without this the player has to close the
      // autocomplete and rely on a follow-up click to see the verdict.
      if (res.ended || store.isOver) {
        await fetchEndView();
      }
    } catch (err) {
      if (err instanceof ApiError) {
        playError = err.message || m.error_play();
      } else {
        playError = m.error_play();
      }
    } finally {
      selectedCell = null;
    }
  };

  const fetchEndView = async (): Promise<void> => {
    const state = store.state;
    if (!state) return;
    try {
      const view = await api.get("/api/games/{gameId}/result", { gameId: state.gameId });
      endGameView = view;
    } catch {
      endGameView = null;
    }
    endGameOpen = true;
    store.endGame();
  };

  const onAbandon = async (): Promise<void> => {
    const state = store.state;
    if (!state) return;
    if (!confirm(m.abandon_confirm())) return;
    try {
      const view = await api.post("/api/games/{gameId}/abandon", { gameId: state.gameId });
      endGameView = view;
    } catch {
      endGameView = null;
    }
    store.endGame();
    endGameOpen = true;
  };

  const cellLabelFor = (cell: Cell | null): string => {
    if (!cell) return "";
    return m.cell_label({ row: cell.row + 1, col: cell.col + 1 });
  };

  const candidatesCountFor = (cell: Cell | null): number | undefined => {
    if (!cell) return undefined;
    const counts = store.grid?.candidatesCount;
    if (!Array.isArray(counts) || counts.length !== 9) return undefined;
    return counts[cell.row * 3 + cell.col];
  };

  const grid = $derived<PublicGrid | null>(store.grid);
  const won = $derived(
    store.state ? store.state.mistakesLeft > 0 && store.state.answers.length === 9 : false,
  );

  // Row/col marker letters are localised — "L" in fr, "R" in en. We pull them
  // from the existing short_row / short_col templates so paraglide stays the
  // single source of truth.
  const rowMarker = $derived(m.row_short({ row: "" }).replace(/[^A-Za-z]/g, "") || "L");
  const colMarker = $derived(m.col_short({ col: "" }).replace(/[^A-Za-z]/g, "") || "C");

  // Today's date — pretty-printed in the user's locale (no spoiler about the
  // puzzle itself, just the publication date).
  const todayLabel = $derived.by(() => {
    const locale = currentLocale() === "en" ? "en-GB" : "fr-FR";
    return new Date().toLocaleDateString(locale, {
      weekday: "long",
      day: "numeric",
      month: "long",
    });
  });

  const mistakesUsed = $derived(
    store.state ? store.state.mistakesAllowed - store.state.mistakesLeft : 0,
  );
  const mistakesAllowed = $derived(store.state?.mistakesAllowed ?? 3);
  const solved = $derived(store.state?.answers.length ?? 0);
  const inGame = $derived(store.state !== null);

  $effect(() => {
    if (typeof window === "undefined") return;
    void loadGrid();
  });
</script>

<section class="kd-game flex flex-col gap-5">
  <!-- Editorial masthead: domain eyebrow + display title + date + rules link.
       Restraint is the design — no shadowed hero card, just typography. -->
  <header class="flex flex-col gap-3">
    <div class="flex items-center justify-between gap-3">
      <p class="eyebrow">
        <span aria-hidden="true" class="text-accent">{m.grid_today()}</span>
        <span aria-hidden="true" class="text-fg-muted/40 mx-1">/</span>
        <span class="text-fg-muted">{m.grid_domain_paris_metro()}</span>
      </p>
      <button
        type="button"
        class="ring-border text-fg-subtle hover:text-fg hover:ring-fg/30 inline-flex h-9 items-center gap-1.5 rounded-md px-2.5 text-xs font-medium ring-1 transition-colors"
        onclick={() => (rulesOpen = true)}
      >
        <BookOpen size={14} aria-hidden="true" />
        <span>{m.rules_button()}</span>
      </button>
    </div>
    <div class="flex items-end justify-between gap-3">
      <h1 class="font-display text-fg text-3xl font-semibold leading-[1.05] sm:text-4xl">
        {m.grid_today()}<span class="text-accent">.</span>
      </h1>
      <time class="text-fg-muted shrink-0 text-xs uppercase tracking-wider">{todayLabel}</time>
    </div>
  </header>

  <!-- Status strip: visible only after the player started a game. Predicate
       help, score and mistake "dots" sit in their own line. Both predicates
       counters are deliberately kept on one row so the player's eye stays on
       the grid below. -->
  {#if inGame}
    <div class="surface flex items-center gap-4 rounded-xl px-3.5 py-2.5">
      <div class="flex flex-col">
        <span class="eyebrow">{m.score()}</span>
        <span class="font-display tabular-nums text-fg text-xl font-semibold leading-none">
          {store.state?.score ?? 0}
        </span>
      </div>
      <div class="bg-border h-9 w-px" aria-hidden="true"></div>
      <div class="flex flex-col">
        <span class="eyebrow">{m.errors()}</span>
        <div class="mt-1 flex items-center gap-1" aria-hidden="true">
          {#each Array(mistakesAllowed) as _, i (i)}
            <span class="kd-dot" class:used={i < mistakesUsed}></span>
          {/each}
          <span class="text-fg-muted ml-1 text-xs tabular-nums">
            {mistakesUsed}/{mistakesAllowed}
          </span>
        </div>
        <span class="sr-only">{mistakesUsed} / {mistakesAllowed}</span>
      </div>
      <div class="ml-auto flex flex-col items-end">
        <span class="eyebrow">{m.modal_endgame_solved_label()}</span>
        <span class="font-display tabular-nums text-fg text-xl font-semibold leading-none">
          {solved}<span class="text-fg-muted/60 text-base font-normal">/9</span>
        </span>
      </div>
    </div>
  {/if}

  {#if gridLoading}
    <div class="kd-grid kd-grid--skeleton" aria-hidden="true">
      <div></div>
      {#each [0, 1, 2] as i (i)}
        <div class="kd-skel kd-skel--chip"></div>
      {/each}
      {#each [0, 1, 2] as r (r)}
        <div class="kd-skel kd-skel--chip"></div>
        {#each [0, 1, 2] as c (c)}
          <div class="kd-skel kd-skel--cell"></div>
        {/each}
      {/each}
    </div>
    <p class="sr-only">{m.loading_stations()}</p>
  {:else if gridError}
    <div
      class="border-danger/30 bg-danger-soft/40 rounded-xl border p-4 text-sm"
      role="alert"
      aria-live="polite"
    >
      <p class="text-danger font-semibold">{gridError}</p>
      <p class="text-fg-subtle mt-1 text-xs">{m.error_grid_unavailable_hint()}</p>
      <button
        type="button"
        class="text-danger ring-danger/40 hover:bg-danger/10 mt-3 inline-flex items-center rounded-md px-2.5 py-1 text-xs font-semibold ring-1 transition-colors"
        onclick={() => void loadGrid()}
      >
        {m.retry_button()}
      </button>
    </div>
  {:else if grid}
    {@const cols = grid.cols}
    {@const rows = grid.rows}
    <div class="kd-grid">
      <!-- Top-left blank "corner" — kept for the column header to align under
           the row chips. Visually filled with a subtle marker to anchor the eye. -->
      <div class="kd-corner" aria-hidden="true">
        <span class="eyebrow">L · C</span>
      </div>
      {#each cols as col, ci (col.id)}
        <PredicateChip predicate={col} orientation="col" index={ci + 1} marker={colMarker} />
      {/each}
      {#each rows as row, ri (row.id)}
        <PredicateChip predicate={row} orientation="row" index={ri + 1} marker={rowMarker} />
        {#each [0, 1, 2] as ci (ci)}
          <CellButton
            row={ri}
            col={ci}
            answer={store.answersByCell.get(cellKey({ row: ri, col: ci }))}
            wrong={wrongCells.has(cellKey({ row: ri, col: ci }))}
            disabled={gridLoading || store.isOver || starting}
            onSelect={onCellSelect}
          />
        {/each}
      {/each}
    </div>

    {#if startError}
      <p class="text-danger text-sm" role="alert">{startError}</p>
    {/if}
    {#if playError}
      <p class="text-danger text-sm" role="alert">{playError}</p>
    {/if}

    <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
      {#if !store.state}
        <button
          type="button"
          class="btn btn-primary w-full sm:w-auto"
          disabled={starting}
          onclick={() => void startGame()}
        >
          <Play size={16} aria-hidden="true" />
          <span>{m.play_button()}</span>
        </button>
        <p class="text-fg-muted text-xs sm:text-right">
          {m.site_lede()}
        </p>
      {:else if !store.isOver}
        <button
          type="button"
          class="btn btn-danger-ghost w-full sm:w-auto"
          onclick={() => void onAbandon()}
        >
          <Flag size={16} aria-hidden="true" />
          <span>{m.abandon_button()}</span>
        </button>
      {:else}
        <button
          type="button"
          class="btn btn-primary w-full sm:w-auto"
          onclick={() => (endGameOpen = true)}
        >
          <BookOpen size={16} aria-hidden="true" />
          <span>{m.see_solutions()}</span>
        </button>
      {/if}
    </div>
  {/if}
</section>

<AutocompleteModal
  open={selectedCell !== null}
  {domain}
  cellLabel={cellLabelFor(selectedCell)}
  candidatesCount={candidatesCountFor(selectedCell)}
  onClose={closeAutocomplete}
  onSubmit={submitAnswer}
/>

<RulesModal open={rulesOpen} onClose={() => (rulesOpen = false)} />

{#if store.state}
  <EndGameModal
    open={endGameOpen}
    {won}
    score={store.state.score}
    maxScore={undefined}
    mistakes={store.state.mistakesAllowed - store.state.mistakesLeft}
    mistakesAllowed={store.state.mistakesAllowed}
    answers={store.state.answers}
    {endGameView}
    {domain}
    gridId={store.state.gridId}
    onClose={() => (endGameOpen = false)}
    onSeeSolutions={() => {
      // already shown — keep closed action minimal
      endGameOpen = true;
    }}
  />
{/if}

<style>
  /* The grid is the page's anchor — gap kept tight so the predicate strips
     read as a single label cluster, with a slightly larger gap on >sm. */
  .kd-grid {
    display: grid;
    /* Wider row-chip column on phone so 4-line French predicates breathe. */
    grid-template-columns: minmax(96px, 1.1fr) repeat(3, minmax(0, 1fr));
    gap: 6px;
  }
  @media (min-width: 640px) {
    .kd-grid {
      grid-template-columns: minmax(0, 0.7fr) repeat(3, minmax(0, 1fr));
      gap: 10px;
    }
  }
  /* Skeleton pulse on initial fetch — paper-pulse, not white-on-white shimmer */
  .kd-skel {
    background: color-mix(in oklab, var(--color-fg) 6%, var(--color-bg-subtle));
    border-radius: var(--radius-md);
    animation: kd-pulse 1.4s var(--ease-in-out) infinite;
  }
  .kd-skel--chip {
    min-height: 64px;
  }
  .kd-skel--cell {
    aspect-ratio: 1 / 1;
    border-radius: var(--radius-lg);
  }
  .kd-grid--skeleton {
    pointer-events: none;
  }
  /* Top-left "L · C" hint corner — gives the grid an editorial table feel. */
  .kd-corner {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-md);
    background: color-mix(in oklab, var(--color-accent) 10%, transparent);
    color: color-mix(in oklab, var(--color-accent) 75%, var(--color-fg-subtle));
    min-height: 64px;
  }
  /* Mistake dots: filled = used, hollow = remaining. Same hue as danger so
     the player feels the warning rise as dots light up. */
  .kd-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: color-mix(in oklab, var(--color-fg) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--color-fg) 18%, transparent);
    transition:
      background-color 200ms var(--ease-out),
      border-color 200ms var(--ease-out);
  }
  .kd-dot.used {
    background: var(--color-danger);
    border-color: var(--color-danger);
  }
  /* Shared button system — defined locally to the game so the design language
     doesn't leak. Keep min-height 44 for touch targets. */
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.7rem 1rem;
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    font-weight: 600;
    min-height: 44px;
    border: 1px solid transparent;
    transition:
      background-color 160ms var(--ease-out),
      color 160ms var(--ease-out),
      transform 160ms var(--ease-out),
      box-shadow 160ms var(--ease-out);
  }
  .btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--color-ring);
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .btn-primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    box-shadow: var(--shadow-paper);
  }
  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: var(--shadow-lift);
  }
  .btn-danger-ghost {
    background: transparent;
    color: var(--color-danger);
    border-color: color-mix(in oklab, var(--color-danger) 45%, transparent);
  }
  .btn-danger-ghost:hover:not(:disabled) {
    background: color-mix(in oklab, var(--color-danger) 8%, transparent);
  }
  @keyframes kd-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.55;
    }
  }
</style>
