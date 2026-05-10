<script lang="ts">
  import * as m from "../../paraglide/messages.js";
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

  const ariaLabel = $derived(
    answer
      ? m.cell_label({ row: row + 1, col: col + 1 }) +
          ", " +
          m.cell_filled({ entity: answer.entityName })
      : m.cell_label({ row: row + 1, col: col + 1 }) + ", " + m.cell_empty(),
  );
</script>

<button
  type="button"
  class="bg-bg-card border-border focus-visible:outline-accent flex aspect-square w-full items-center justify-center rounded-md border p-2 text-center text-sm font-medium transition focus:outline-none disabled:opacity-60"
  class:filled={answer !== undefined}
  class:wrong
  aria-label={ariaLabel}
  disabled={disabled || answer !== undefined}
  onclick={handleClick}
>
  {#if answer}
    <span class="line-clamp-3 break-words text-fg">{answer.entityName}</span>
  {:else}
    <span aria-hidden="true" class="text-fg-muted text-2xl">+</span>
  {/if}
</button>

<style>
  button {
    min-height: 44px;
    min-width: 44px;
  }
  .filled {
    background: color-mix(in oklab, var(--color-success) 10%, var(--color-bg-card));
    border-color: color-mix(in oklab, var(--color-success) 35%, var(--color-border));
  }
  /* Wrong-answer flash — short shake + red border so the player feels the
     mistake even though the cell stays empty (re-essai authorized). */
  .wrong {
    animation: kd-shake 450ms ease-in-out;
    border-color: var(--color-danger);
    background: color-mix(in oklab, var(--color-danger) 12%, var(--color-bg-card));
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
    .wrong {
      animation: none;
    }
  }
</style>
