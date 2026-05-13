<script lang="ts">
  import Check from "lucide-svelte/icons/check";
  import Plus from "lucide-svelte/icons/plus";
  import * as m from "../../paraglide/messages.js";
  import { rarityFor, type Rarity } from "../rarity.js";
  import type { CellAnswer } from "../stores/gameStore.svelte.js";

  interface Props {
    row: number;
    col: number;
    answer: CellAnswer | undefined;
    wrong?: boolean;
    disabled: boolean;
    onSelect: (row: number, col: number) => void;
  }

  let { row, col, answer, wrong = false, disabled, onSelect }: Props = $props();

  const handleClick = (): void => {
    onSelect(row, col);
  };

  const cellLabel = $derived(m.cell_label({ row: row + 1, col: col + 1 }));
  const ariaLabel = $derived(
    answer
      ? cellLabel + ", " + m.cell_filled({ entity: answer.entityName })
      : cellLabel + ", " + m.cell_empty(),
  );
  const shortLabel = $derived(m.cell_short_label({ row: row + 1, col: col + 1 }));

  // The persistent rarity badge sits in the top-right corner of a solved
  // cell. We only render it when the cell is solved AND the rarity is more
  // notable than 'common' — common stations don't need decoration, the
  // badge would mostly add noise.
  const rarity: Rarity | null = $derived(answer ? rarityFor(answer.fameScore) : null);
  const rarityLabel = $derived.by((): string => {
    switch (rarity) {
      case "rare":
        return m.rarity_rare();
      case "epic":
        return m.rarity_epic();
      case "legendary":
        return m.rarity_legendary();
      default:
        return "";
    }
  });
</script>

<button
  type="button"
  class="cell"
  class:filled={answer !== undefined}
  class:wrong
  aria-label={ariaLabel}
  data-filled={answer !== undefined}
  disabled={disabled || answer !== undefined}
  onclick={handleClick}
>
  <span class="cell-tag eyebrow" aria-hidden="true">{shortLabel}</span>

  {#if answer}
    {#if rarity && rarity !== "common"}
      <!-- Persistent corner badge — replaces the plain check on cells where
           the resolved entity is at least 'Rare'. Common cells keep the
           understated check so the badge wall doesn't drown the eye. -->
      <span
        class="cell-rarity"
        class:rarity-rare={rarity === "rare"}
        class:rarity-epic={rarity === "epic"}
        class:rarity-legendary={rarity === "legendary"}
        aria-label={rarityLabel}
        title={rarityLabel}
      >
        {rarityLabel}
      </span>
    {:else}
      <span class="cell-check" aria-hidden="true">
        <Check size={14} strokeWidth={2.5} />
      </span>
    {/if}
    <span class="cell-answer" title={answer.entityName}>{answer.entityName}</span>
  {:else}
    <span class="cell-plus" aria-hidden="true">
      <Plus size={22} strokeWidth={1.6} />
    </span>
  {/if}
</button>

<style>
  .cell {
    /* Editorial transit panel — square card with a corner index, a satisfying
       hover lift and a left "line bar" that animates upward when filled. */
    --bar-color: var(--color-accent);
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    aspect-ratio: 1 / 1;
    width: 100%;
    padding: 0.5rem;
    border-radius: var(--radius-lg);
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    color: var(--color-fg);
    font-size: 0.8125rem;
    line-height: 1.15;
    font-weight: 500;
    overflow: hidden;
    transition:
      transform 160ms var(--ease-out),
      border-color 160ms var(--ease-out),
      background-color 200ms var(--ease-out),
      box-shadow 200ms var(--ease-out);
    box-shadow: var(--shadow-paper);
    min-height: 44px;
    min-width: 44px;
  }
  .cell:hover:not(:disabled) {
    border-color: var(--color-border-strong);
    transform: translateY(-1px);
    box-shadow: var(--shadow-lift);
  }
  .cell:active:not(:disabled) {
    transform: translateY(0);
  }
  .cell:focus-visible {
    outline: none;
    border-color: var(--color-accent);
    box-shadow:
      0 0 0 3px var(--color-ring),
      var(--shadow-paper);
  }
  .cell:disabled {
    cursor: default;
  }
  /* Index tag in the top-left corner — like a station number on a wall sign. */
  .cell-tag {
    position: absolute;
    top: 0.45rem;
    left: 0.55rem;
    font-size: 9.5px;
    letter-spacing: 0.18em;
    color: var(--color-fg-muted);
    opacity: 0.85;
  }
  .cell-plus {
    color: color-mix(in oklab, var(--color-fg-muted) 80%, transparent);
    transition: transform 200ms var(--ease-out);
  }
  .cell:hover:not(:disabled) .cell-plus {
    transform: rotate(45deg);
    color: var(--color-accent);
  }
  .cell-check {
    position: absolute;
    top: 0.45rem;
    right: 0.5rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.1rem;
    height: 1.1rem;
    border-radius: 999px;
    color: var(--color-success);
    background: color-mix(in oklab, var(--color-success) 18%, transparent);
  }
  /* Rarity badge — sits where the check normally lives on solved cells.
     Three colour stops keyed off MMO loot ladders (blue / purple / gold).
     Common cells fall back to .cell-check above, no badge. */
  .cell-rarity {
    position: absolute;
    top: 0.4rem;
    right: 0.45rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 16px;
    padding: 0 6px;
    border-radius: 999px;
    font-size: 9px;
    line-height: 1;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border: 1px solid currentColor;
    background: color-mix(in oklab, currentColor 14%, transparent);
  }
  .rarity-rare {
    color: oklch(0.6 0.16 250);
  }
  .rarity-epic {
    color: oklch(0.58 0.18 305);
  }
  .rarity-legendary {
    color: oklch(0.7 0.16 75);
    box-shadow: 0 0 0 1px color-mix(in oklab, currentColor 35%, transparent);
  }
  .cell-answer {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-align: center;
    font-weight: 600;
    font-size: 0.8rem;
    line-height: 1.15;
    color: var(--color-fg);
    padding: 0 0.25rem;
    overflow-wrap: anywhere;
    hyphens: auto;
  }
  /* Filled state — paper turns a faint success cream, a vertical line bar
     "rises" from the bottom (transit-line accent), and the entity name swaps
     in with a soft fade-up. */
  .cell.filled {
    background: color-mix(in oklab, var(--color-success) 8%, var(--color-bg-card));
    border-color: color-mix(in oklab, var(--color-success) 28%, var(--color-border));
    cursor: default;
  }
  .cell.filled::before {
    content: "";
    position: absolute;
    left: 6px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 999px;
    background: var(--color-success);
    transform-origin: bottom;
    animation: kd-bar-grow 320ms var(--ease-out) forwards;
  }
  .cell.filled .cell-answer {
    animation: kd-fade-up 280ms var(--ease-out) both;
  }
  /* Wrong-answer flash — short shake + red border so the player feels the
     mistake even though the cell stays empty (re-essai authorized). */
  .cell.wrong {
    animation: kd-shake 450ms ease-in-out;
    border-color: var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 12%, var(--color-bg-card));
  }
  .cell.wrong .cell-plus {
    color: var(--color-danger);
  }
  @keyframes kd-bar-grow {
    from {
      transform: scaleY(0);
    }
    to {
      transform: scaleY(1);
    }
  }
  @keyframes kd-fade-up {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @keyframes kd-shake {
    0%,
    100% {
      transform: translateX(0);
    }
    20% {
      transform: translateX(-4px);
    }
    40% {
      transform: translateX(4px);
    }
    60% {
      transform: translateX(-3px);
    }
    80% {
      transform: translateX(2px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .cell.wrong,
    .cell.filled::before,
    .cell.filled .cell-answer {
      animation: none;
    }
  }
</style>
