<script lang="ts">
  import X from "lucide-svelte/icons/x";
  import * as m from "../../paraglide/messages.js";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open, onClose }: Props = $props();

  const STORAGE_KEY = "kalidoku.rules.hide";

  let hide = $state<boolean>(false);

  $effect(() => {
    if (typeof window === "undefined") return;
    if (open) {
      hide = window.localStorage.getItem(STORAGE_KEY) === "1";
    } else {
      window.localStorage.setItem(STORAGE_KEY, hide ? "1" : "0");
    }
  });

  const handleBackdrop = (e: MouseEvent): void => {
    if (e.target === e.currentTarget) onClose();
  };

  const handleKey = (e: KeyboardEvent): void => {
    if (e.key === "Escape") onClose();
  };

  type Step = { n: number; body: string };
  const steps = $derived<Step[]>([
    { n: 1, body: m.modal_rules_step_1() },
    { n: 2, body: m.modal_rules_step_2() },
    { n: 3, body: m.modal_rules_step_3() },
  ]);
</script>

<svelte:window onkeydown={handleKey} />

{#if open}
  <div class="kd-modal-backdrop" role="presentation" onclick={handleBackdrop}>
    <div class="kd-modal" role="dialog" aria-modal="true" aria-labelledby="rules-title">
      <header class="flex items-start justify-between gap-3 pb-3">
        <div>
          <p class="eyebrow text-accent">{m.rules_button()}</p>
          <h2
            id="rules-title"
            class="font-display text-fg mt-1 text-2xl font-semibold leading-tight"
          >
            {m.modal_rules_title()}
          </h2>
        </div>
        <button type="button" class="kd-modal-close" aria-label={m.modal_close()} onclick={onClose}>
          <X size={18} aria-hidden="true" />
        </button>
      </header>

      <p class="text-fg-subtle text-sm leading-relaxed">{m.modal_rules_intro()}</p>

      <ol class="mt-4 flex flex-col gap-2.5">
        {#each steps as step (step.n)}
          <li class="kd-step">
            <span class="kd-step__n font-display" aria-hidden="true">{step.n}</span>
            <p class="text-fg-subtle text-sm leading-snug">{step.body}</p>
          </li>
        {/each}
      </ol>

      <div class="kd-legend mt-5 rounded-lg p-3.5">
        <p class="eyebrow text-fg-muted">{m.modal_rules_legend_heading()}</p>
        <p class="text-fg-subtle mt-1.5 text-xs leading-relaxed">{m.modal_rules_legend_body()}</p>
      </div>

      <label class="text-fg-subtle mt-5 flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          class="kd-checkbox size-4 rounded border-border accent-accent"
          bind:checked={hide}
        />
        <span>{m.modal_rules_dont_show()}</span>
      </label>
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
    width: 100%;
    max-width: 28rem;
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xl);
    padding: 1.25rem 1.25rem 1.5rem;
    box-shadow: var(--shadow-modal);
    animation: kd-slide-up 220ms var(--ease-out);
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
  .kd-step {
    display: flex;
    align-items: flex-start;
    gap: 0.85rem;
    padding: 0.6rem 0.75rem;
    border-radius: var(--radius-md);
    background: var(--color-bg-subtle);
  }
  .kd-step__n {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 1.85rem;
    height: 1.85rem;
    border-radius: 999px;
    background: var(--color-accent);
    color: var(--color-accent-fg);
    font-size: 0.85rem;
    font-weight: 700;
    font-variation-settings:
      "opsz" 14,
      "SOFT" 30;
  }
  .kd-legend {
    background: color-mix(in oklab, var(--color-accent) 6%, transparent);
    border: 1px dashed color-mix(in oklab, var(--color-accent) 35%, transparent);
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
