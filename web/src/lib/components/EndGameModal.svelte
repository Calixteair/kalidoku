<script lang="ts">
  import Check from "lucide-svelte/icons/check";
  import ChevronDown from "lucide-svelte/icons/chevron-down";
  import Copy from "lucide-svelte/icons/copy";
  import Share2 from "lucide-svelte/icons/share-2";
  import Swords from "lucide-svelte/icons/swords";
  import X from "lucide-svelte/icons/x";
  import * as m from "../../paraglide/messages.js";
  import { api, ApiError } from "../api/client.js";
  import type { components } from "../api/types.js";
  import type { CellAnswer } from "../stores/gameStore.svelte.js";
  import { buildShareString, copyToClipboard, pickShareString, shareNative } from "../share.js";

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
    /** Domain + gridId of the game just played. Set when both are available
     *  so the duel CTA can challenge a friend on the same grid. */
    domain?: string | undefined;
    gridId?: string | undefined;
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
    domain,
    gridId,
    onClose,
    onSeeSolutions,
  }: Props = $props();

  let copied = $state(false);
  let showSolutions = $state(false);
  let duelLoading = $state(false);
  let duelUrl = $state<string | null>(null);
  let duelError = $state<string | null>(null);
  let duelCopied = $state(false);

  const cellLabel = (row: number, col: number): string =>
    m.cell_label({ row: row + 1, col: col + 1 });

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

  const handleKey = (e: KeyboardEvent): void => {
    if (open && e.key === "Escape") onClose();
  };

  const handleChallenge = async (): Promise<void> => {
    if (!domain || !gridId || duelLoading) return;
    duelLoading = true;
    duelError = null;
    try {
      const r = await api.post("/api/duels", undefined, { domain, gridId });
      duelUrl = r.shareUrl;
      // Best-effort: try the native share sheet first, then fall back to
      // clipboard so the user always walks away with the link in hand.
      const shared = await shareNative(r.shareUrl, m.duel_share_title());
      if (!shared) {
        const ok = await copyToClipboard(r.shareUrl);
        duelCopied = ok;
        if (ok) setTimeout(() => (duelCopied = false), 2400);
      }
    } catch (err) {
      duelError = err instanceof ApiError ? err.message : m.error_network();
    } finally {
      duelLoading = false;
    }
  };

  const copyDuelUrl = async (): Promise<void> => {
    if (!duelUrl) return;
    const ok = await copyToClipboard(duelUrl);
    duelCopied = ok;
    if (ok) setTimeout(() => (duelCopied = false), 2400);
  };

  const headlineTitle = $derived(won ? m.modal_endgame_won() : m.modal_endgame_lost());
  const headlineSub = $derived(
    won ? m.modal_endgame_won_subtitle() : m.modal_endgame_lost_subtitle(),
  );
  const solved = $derived(answers.length);
  const originality = $derived(endGameView?.summary?.originalityScore ?? 0);
  const originalityVisible = $derived(endGameView !== null && solved > 0);
</script>

<svelte:window onkeydown={handleKey} />

{#if open}
  <div class="kd-modal-backdrop" role="presentation" onclick={handleBackdrop}>
    <div
      class="kd-modal"
      class:kd-modal--won={won}
      class:kd-modal--lost={!won}
      role="dialog"
      aria-modal="true"
      aria-labelledby="endgame-title"
    >
      <header class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="eyebrow" class:text-success={won} class:text-danger={!won}>
            {won ? m.modal_endgame_won() : m.modal_endgame_lost()}
          </p>
          <h2
            id="endgame-title"
            class="font-display text-fg mt-1 text-3xl font-semibold leading-tight"
          >
            {headlineTitle}
          </h2>
          <p class="text-fg-subtle mt-1 text-sm">{headlineSub}</p>
        </div>
        <button type="button" class="kd-modal-close" aria-label={m.modal_close()} onclick={onClose}>
          <X size={18} aria-hidden="true" />
        </button>
      </header>

      <!-- Stats panel — three big numbers, editorial table look. -->
      <dl class="kd-stats mt-5">
        <div class="kd-stat">
          <dt class="eyebrow">{m.score()}</dt>
          <dd class="font-display tabular-nums">
            {score}{#if maxScore !== undefined}<span class="kd-stat__max">/{maxScore}</span>{/if}
          </dd>
        </div>
        <div class="kd-stat">
          <dt class="eyebrow">{m.modal_endgame_solved_label()}</dt>
          <dd class="font-display tabular-nums">
            {solved}<span class="kd-stat__max">/9</span>
          </dd>
        </div>
        <div class="kd-stat">
          <dt class="eyebrow">{m.errors()}</dt>
          <dd class="font-display tabular-nums" class:text-danger={mistakes >= mistakesAllowed}>
            {mistakes}<span class="kd-stat__max">/{mistakesAllowed}</span>
          </dd>
        </div>
      </dl>

      <!-- Originality — a single horizontal bar with the label above and the
           numeric value to the right. We only surface it when the player solved
           at least one cell so a 0-orig empty grid doesn't feel like a penalty. -->
      {#if originalityVisible}
        <section class="kd-originality mt-4" aria-label={m.originality_label()}>
          <div class="kd-originality__head">
            <p class="eyebrow text-fg-muted">{m.originality_label()}</p>
            <p class="kd-originality__value font-display tabular-nums">
              {originality}<span class="kd-stat__max">/100</span>
            </p>
          </div>
          <div
            class="kd-originality__track"
            role="progressbar"
            aria-valuenow={originality}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-label={m.originality_label()}
          >
            <span class="kd-originality__bar" style="width: {originality}%"></span>
          </div>
          <p class="kd-originality__hint">{m.originality_hint()}</p>
        </section>
      {/if}

      <!-- Share block: the visual hero. Boxed monospace card that mirrors what
           will land in the user's clipboard or share sheet — wysiwyg sharing. -->
      <section class="kd-share mt-5">
        <header class="mb-2 flex items-baseline justify-between gap-2">
          <p class="eyebrow text-fg-muted">{m.modal_endgame_share_heading()}</p>
          {#if copied}
            <span class="text-success inline-flex items-center gap-1 text-xs font-medium">
              <Check size={12} aria-hidden="true" />
              {m.share_copied()}
            </span>
          {/if}
        </header>
        <pre class="kd-share__pre" aria-live="polite">{shareString}</pre>
        <p class="text-fg-muted mt-2 text-[11px] leading-relaxed">{m.modal_endgame_share_hint()}</p>
      </section>

      <div class="mt-4 flex flex-col gap-2 sm:flex-row">
        <button type="button" class="btn btn-primary flex-1" onclick={handleShare}>
          <Share2 size={16} aria-hidden="true" />
          <span>{m.share_button()}</span>
        </button>
        <button type="button" class="btn btn-secondary flex-1" onclick={handleCopy}>
          {#if copied}
            <Check size={16} aria-hidden="true" />
            <span>{m.share_copied()}</span>
          {:else}
            <Copy size={16} aria-hidden="true" />
            <span>{m.copy_result()}</span>
          {/if}
        </button>
      </div>

      <!-- Duel CTA. Only visible once we know the grid the player just
           tackled — otherwise we can't pin the duel to that grid. -->
      {#if domain && gridId}
        <div class="kd-duel-cta mt-3">
          {#if duelUrl === null}
            <button
              type="button"
              class="btn btn-secondary w-full justify-center"
              disabled={duelLoading}
              onclick={handleChallenge}
            >
              <Swords size={16} aria-hidden="true" />
              <span>{duelLoading ? m.duel_share_creating() : m.duel_share_action()}</span>
            </button>
          {:else}
            <div class="border-border bg-bg-subtle flex flex-col gap-2 rounded-lg border p-3">
              <p class="eyebrow text-fg-muted">{m.duel_share_ready()}</p>
              <p class="text-fg break-all text-xs font-mono">{duelUrl}</p>
              <button type="button" class="btn btn-secondary justify-center" onclick={copyDuelUrl}>
                {#if duelCopied}
                  <Check size={14} aria-hidden="true" />
                  <span>{m.share_copied()}</span>
                {:else}
                  <Copy size={14} aria-hidden="true" />
                  <span>{m.copy_result()}</span>
                {/if}
              </button>
            </div>
          {/if}
          {#if duelError}
            <p class="text-danger mt-2 text-xs" role="alert">{duelError}</p>
          {/if}
        </div>
      {/if}

      {#if endGameView?.solutionsByCell?.length}
        <div class="border-border mt-5 border-t pt-4">
          <button
            type="button"
            class="text-fg-subtle hover:text-fg inline-flex items-center gap-1.5 text-sm font-medium transition-colors"
            aria-expanded={showSolutions}
            onclick={() => {
              showSolutions = !showSolutions;
              if (onSeeSolutions) onSeeSolutions();
            }}
          >
            <ChevronDown
              size={16}
              aria-hidden="true"
              class={"chevron transition-transform " + (showSolutions ? "rotate-180" : "")}
            />
            <span>{showSolutions ? m.hide_solutions() : m.see_solutions()}</span>
          </button>

          {#if showSolutions}
            <div class="kd-solutions">
              <ul class="kd-solutions__list">
                {#each endGameView.solutionsByCell as cellSol (cellSol.cell.row * 3 + cellSol.cell.col)}
                  <li class="kd-solutions__item">
                    <p class="eyebrow text-fg-muted">
                      {cellLabel(cellSol.cell.row, cellSol.cell.col)}
                    </p>
                    <ul class="kd-solutions__cands mt-1">
                      {#each cellSol.candidates as cand (cand.id)}
                        <li class="kd-solutions__cand">
                          <span class="text-fg text-sm leading-snug">{cand.name}</span>
                          {#if cand.fameScore !== undefined && cand.fameScore !== null}
                            <span
                              class="kd-fame-chip"
                              title={m.fame_chip_title({ fame: cand.fameScore })}
                            >
                              {cand.fameScore}
                            </span>
                          {/if}
                        </li>
                      {/each}
                    </ul>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .kd-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 0.5rem;
    background: color-mix(in oklab, oklch(0.08 0 0) 55%, transparent);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    animation: kd-fade-in 180ms var(--ease-out);
  }
  @media (min-width: 640px) {
    .kd-modal-backdrop {
      align-items: center;
      padding: 1rem;
    }
  }
  .kd-modal {
    position: relative;
    width: 100%;
    max-width: 28rem;
    max-height: 92vh;
    overflow-y: auto;
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xl);
    padding: 1.25rem 1.25rem 1.5rem;
    box-shadow: var(--shadow-modal);
    animation: kd-slide-up 220ms var(--ease-out);
  }
  /* The very top of the modal gets a thin "ribbon" stripe in the verdict
     colour — quick read for the player when they open it. */
  .kd-modal::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    border-top-left-radius: var(--radius-xl);
    border-top-right-radius: var(--radius-xl);
  }
  .kd-modal--won::before {
    background: linear-gradient(
      90deg,
      var(--color-success) 0%,
      color-mix(in oklab, var(--color-success) 60%, var(--color-accent)) 100%
    );
  }
  .kd-modal--lost::before {
    background: linear-gradient(
      90deg,
      var(--color-danger) 0%,
      color-mix(in oklab, var(--color-danger) 60%, var(--color-fg-muted)) 100%
    );
  }
  @media (min-width: 640px) {
    .kd-modal {
      padding: 1.5rem 1.5rem 1.75rem;
      animation: kd-pop-in 220ms var(--ease-out);
    }
  }
  .kd-modal-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 36px;
    width: 36px;
    color: var(--color-fg-muted);
    border-radius: var(--radius-md);
    transition:
      color 160ms var(--ease-out),
      background-color 160ms var(--ease-out);
  }
  .kd-modal-close:hover {
    color: var(--color-fg);
    background: color-mix(in oklab, var(--color-fg) 8%, transparent);
  }
  /* Stats — three columns at any width (the eyebrow + display number block
     is small enough to fit on 360px). */
  .kd-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    background: var(--color-bg-subtle);
    border-radius: var(--radius-lg);
    padding: 0.85rem 1rem;
    border: 1px solid var(--color-border);
  }
  .kd-stat {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .kd-stat dt {
    font-size: 10px;
  }
  .kd-stat dd {
    font-size: 1.65rem;
    line-height: 1;
    font-weight: 600;
    color: var(--color-fg);
  }
  .kd-stat__max {
    font-size: 0.75rem;
    color: var(--color-fg-muted);
    margin-left: 2px;
    font-family: var(--font-sans);
    font-weight: 500;
    font-variation-settings: normal;
  }
  /* Share block — proudly mono, with a torn-ticket dashed border on top so it
     reads as "a thing to copy/share". */
  .kd-share {
    position: relative;
    background: var(--color-bg-subtle);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 0.85rem 1rem;
  }
  .kd-share__pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--color-fg);
    white-space: pre-wrap;
    word-break: break-word;
  }
  /* Buttons — re-used naming with the same visual rules as in Grid.svelte. */
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
      transform 160ms var(--ease-out),
      box-shadow 160ms var(--ease-out),
      color 160ms var(--ease-out);
  }
  .btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--color-ring);
  }
  .btn-primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    box-shadow: var(--shadow-paper);
  }
  .btn-primary:hover {
    transform: translateY(-1px);
    box-shadow: var(--shadow-lift);
  }
  .btn-secondary {
    background: var(--color-bg-card);
    color: var(--color-fg);
    border-color: var(--color-border);
  }
  .btn-secondary:hover {
    border-color: var(--color-border-strong);
  }
  /* Solutions accordion */
  .kd-solutions {
    margin-top: 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    max-height: 18rem;
    overflow-y: auto;
  }
  .kd-solutions__list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .kd-solutions__item {
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid var(--color-border);
  }
  .kd-solutions__item:last-child {
    border-bottom: none;
  }
  .kd-solutions__cands {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 0.6rem;
  }
  .kd-solutions__cand {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
  }
  /* Originality block — flat bar with accent fill and a tiny hint underneath. */
  .kd-originality {
    background: var(--color-bg-subtle);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 0.7rem 0.95rem 0.8rem;
  }
  .kd-originality__head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .kd-originality__value {
    font-size: 1.1rem;
    line-height: 1;
    font-weight: 600;
    color: var(--color-fg);
  }
  .kd-originality__track {
    margin-top: 0.5rem;
    height: 6px;
    background: color-mix(in oklab, var(--color-fg) 8%, transparent);
    border-radius: 999px;
    overflow: hidden;
  }
  .kd-originality__bar {
    display: block;
    height: 100%;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--color-accent) 70%, var(--color-fg)) 0%,
      var(--color-accent) 100%
    );
    border-radius: inherit;
    transition: width 360ms var(--ease-out);
  }
  @media (prefers-reduced-motion: reduce) {
    .kd-originality__bar {
      transition: none;
    }
  }
  .kd-originality__hint {
    margin-top: 0.4rem;
    font-size: 11px;
    line-height: 1.4;
    color: var(--color-fg-muted);
  }
  /* Fame chip — tiny pill that says "this entity is roughly N/100 famous".
     Low fame = strong accent (you picked a niche one); high fame = subdued. */
  .kd-fame-chip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    padding: 0 6px;
    height: 18px;
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    border-radius: 999px;
    background: color-mix(in oklab, var(--color-accent) 18%, transparent);
    color: color-mix(in oklab, var(--color-accent) 70%, var(--color-fg));
    border: 1px solid color-mix(in oklab, var(--color-accent) 30%, transparent);
  }
  @keyframes kd-fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  @keyframes kd-slide-up {
    from {
      transform: translateY(16px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
  @keyframes kd-pop-in {
    from {
      transform: scale(0.96);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .kd-modal-backdrop,
    .kd-modal {
      animation: none;
    }
  }
</style>
