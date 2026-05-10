<script lang="ts">
  import * as m from "../../paraglide/messages.js";
  import type { components } from "../api/types.js";
  import type { CellAnswer } from "../stores/gameStore.svelte.js";
  import { buildShareString, copyToClipboard, pickShareString, shareNative } from "../share.js";
  import { formatMessage } from "../i18n.js";

  type EndGameView = components["schemas"]["EndGameView"];

  interface Props {
    open: boolean;
    won: boolean;
    score: number;
    maxScore: number | undefined;
    mistakes: number;
    mistakesAllowed: number;
    answers: CellAnswer[];
    endGameView: EndGameView | null;
    onClose: () => void;
    onSeeSolutions?: (() => void) | undefined;
  }

  let {
    open,
    won,
    score,
    maxScore,
    mistakes,
    mistakesAllowed,
    answers,
    endGameView,
    onClose,
    onSeeSolutions,
  }: Props = $props();

  let copied = $state(false);

  const today = (): string => {
    return new Date().toISOString().slice(0, 10);
  };

  const shareString = $derived(
    pickShareString(endGameView?.summary ?? null, endGameView, () =>
      buildShareString({
        date: today(),
        score,
        maxScore,
        mistakes,
        mistakesAllowed,
        answers,
      }),
    ),
  );

  const handleShare = async (): Promise<void> => {
    const title = m.share_template({
      date: today(),
      score,
      max: maxScore ?? 9,
    });
    const ok = await shareNative(shareString, title);
    if (!ok) {
      const c = await copyToClipboard(shareString);
      copied = c;
      if (c) {
        setTimeout(() => (copied = false), 2000);
      }
    }
  };

  const handleCopy = async (): Promise<void> => {
    const ok = await copyToClipboard(shareString);
    copied = ok;
    if (ok) setTimeout(() => (copied = false), 2000);
  };

  const handleBackdrop = (e: MouseEvent): void => {
    if (e.target === e.currentTarget) onClose();
  };
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-black/40 p-2 sm:p-4"
    role="presentation"
    onclick={handleBackdrop}
  >
    <div
      class="bg-bg-card border-border flex w-full max-w-md flex-col gap-3 rounded-lg border p-4 shadow-xl"
      role="dialog"
      aria-modal="true"
      aria-labelledby="endgame-title"
    >
      <header class="flex items-start justify-between gap-2">
        <div>
          <h2 id="endgame-title" class="text-lg font-semibold">
            {won ? m.modal_endgame_won() : m.modal_endgame_lost()}
          </h2>
          <p class="text-fg-muted text-sm">
            {m.score()}: {score}{maxScore !== undefined ? `/${maxScore}` : ""}
            · {m.errors()}: {mistakes}/{mistakesAllowed}
          </p>
        </div>
        <button
          type="button"
          class="text-fg-muted hover:text-fg flex h-9 w-9 items-center justify-center rounded-md"
          aria-label={m.modal_close()}
          onclick={onClose}
        >
          ×
        </button>
      </header>

      <pre
        class="bg-bg-subtle text-fg-subtle whitespace-pre-wrap rounded-md p-3 font-mono text-sm">{shareString}</pre>

      <div class="flex flex-col gap-2 sm:flex-row">
        <button
          type="button"
          class="bg-accent text-accent-fg flex-1 rounded-md px-4 py-2 text-sm font-semibold"
          onclick={handleShare}
        >
          {m.share_button()}
        </button>
        <button
          type="button"
          class="border-border bg-bg-card text-fg flex-1 rounded-md border px-4 py-2 text-sm font-semibold"
          onclick={handleCopy}
        >
          {copied ? m.share_copied() : m.copy_result()}
        </button>
      </div>

      {#if onSeeSolutions}
        <button
          type="button"
          class="text-fg-subtle text-sm underline-offset-2 hover:underline"
          onclick={onSeeSolutions}
        >
          {m.see_solutions()}
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  button {
    min-height: 44px;
  }
</style>
