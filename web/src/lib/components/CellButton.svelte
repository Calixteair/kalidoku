<script lang="ts">
  import * as m from "../../paraglide/messages.js";
  import { formatMessage } from "../i18n.js";
  import type { CellAnswer } from "../stores/gameStore.svelte.js";

  interface Props {
    row: number;
    col: number;
    answer: CellAnswer | undefined;
    disabled: boolean;
    onSelect: (row: number, col: number) => void;
  }

  let { row, col, answer, disabled, onSelect }: Props = $props();

  const handleClick = (): void => {
    onSelect(row, col);
  };

  const ariaLabel = $derived(
    answer
      ? formatMessage(m.cell_label(), { row: row + 1, col: col + 1 }) +
          ", " +
          formatMessage(m.cell_filled(), { entity: answer.entityName })
      : formatMessage(m.cell_label(), { row: row + 1, col: col + 1 }) + ", " + m.cell_empty(),
  );
</script>

<button
  type="button"
  class="bg-bg-card border-border focus-visible:outline-accent flex aspect-square w-full items-center justify-center rounded-md border p-2 text-center text-sm font-medium transition focus:outline-none disabled:opacity-60"
  class:filled={answer !== undefined}
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
</style>
