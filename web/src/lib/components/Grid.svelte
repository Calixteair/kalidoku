<script lang="ts">
  import * as m from "../../paraglide/messages.js";
  import { api, ApiError } from "../api/client.js";
  import type { components } from "../api/types.js";
  import AutocompleteModal from "./AutocompleteModal.svelte";
  import CellButton from "./CellButton.svelte";
  import EndGameModal from "./EndGameModal.svelte";
  import PredicateChip from "./PredicateChip.svelte";
  import RulesModal from "./RulesModal.svelte";
  import { createGameStore, cellKey } from "../stores/gameStore.svelte.js";
  import { formatMessage } from "../i18n.js";

  type PublicGrid = components["schemas"]["PublicGrid"];
  type Cell = components["schemas"]["Cell"];
  type EndGameView = components["schemas"]["EndGameView"];

  interface Props {
    domain: string;
  }

  let { domain }: Props = $props();

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

  const loadGrid = async (): Promise<void> => {
    gridLoading = true;
    gridError = null;
    try {
      const g = await api.get("/api/grids/{domain}/today", { domain }, { locale: "fr" });
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
        mode: "daily",
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
    return formatMessage(m.cell_label(), { row: cell.row + 1, col: cell.col + 1 });
  };

  const grid = $derived<PublicGrid | null>(store.grid);
  const won = $derived(
    store.state ? store.state.mistakesLeft > 0 && store.state.answers.length === 9 : false,
  );

  $effect(() => {
    if (typeof window === "undefined") return;
    void loadGrid();
  });
</script>

<section class="flex flex-col gap-4">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <div class="flex flex-col">
      <h2 class="text-lg font-semibold tracking-tight md:text-xl">{m.grid_today()}</h2>
      {#if store.state}
        <p class="text-fg-muted text-sm">
          {m.score()}: <strong class="text-fg">{store.state.score}</strong>
          · {m.errors()}:
          <strong class="text-fg"
            >{store.state.mistakesAllowed - store.state.mistakesLeft}/{store.state
              .mistakesAllowed}</strong
          >
        </p>
      {/if}
    </div>
    <button
      type="button"
      class="border-border bg-bg-card text-fg-subtle rounded-md border px-3 py-1 text-sm"
      onclick={() => (rulesOpen = true)}
    >
      {m.rules_button()}
    </button>
  </div>

  {#if gridLoading}
    <p class="text-fg-muted text-sm">{m.loading_stations()}</p>
  {:else if gridError}
    <p class="text-danger rounded-md border border-danger/30 bg-danger/5 p-3 text-sm" role="alert">
      {gridError}
    </p>
  {:else if grid}
    {@const cols = grid.cols}
    {@const rows = grid.rows}
    <div
      class="grid gap-1.5"
      style="grid-template-columns: minmax(0, 0.65fr) repeat(3, minmax(0, 1fr));"
    >
      <div></div>
      {#each cols as col (col.id)}
        <PredicateChip predicate={col} />
      {/each}
      {#each rows as row, ri (row.id)}
        <PredicateChip predicate={row} />
        {#each [0, 1, 2] as ci (ci)}
          <CellButton
            row={ri}
            col={ci}
            answer={store.answersByCell.get(cellKey({ row: ri, col: ci }))}
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

    <div class="flex flex-col gap-2 sm:flex-row sm:justify-between">
      {#if !store.state}
        <button
          type="button"
          class="bg-accent text-accent-fg rounded-md px-4 py-2 text-sm font-semibold disabled:opacity-60"
          disabled={starting}
          onclick={() => void startGame()}
        >
          {m.play_button()}
        </button>
      {:else if !store.isOver}
        <button
          type="button"
          class="text-danger border-danger/40 hover:bg-danger/5 rounded-md border bg-transparent px-4 py-2 text-sm font-semibold"
          onclick={() => void onAbandon()}
        >
          {m.abandon_button()}
        </button>
      {:else}
        <button
          type="button"
          class="bg-accent text-accent-fg rounded-md px-4 py-2 text-sm font-semibold"
          onclick={() => (endGameOpen = true)}
        >
          {m.see_solutions()}
        </button>
      {/if}
    </div>
  {/if}
</section>

<AutocompleteModal
  open={selectedCell !== null}
  {domain}
  cellLabel={cellLabelFor(selectedCell)}
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
    onClose={() => (endGameOpen = false)}
    onSeeSolutions={() => {
      // already shown — keep closed action minimal
      endGameOpen = true;
    }}
  />
{/if}

<style>
  button {
    min-height: 44px;
  }
</style>
