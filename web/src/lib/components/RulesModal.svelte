<script lang="ts">
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
</script>

<svelte:window onkeydown={handleKey} />

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-black/40 p-2 sm:p-4"
    role="presentation"
    onclick={handleBackdrop}
  >
    <div
      class="bg-bg-card border-border w-full max-w-md rounded-lg border p-4 shadow-xl"
      role="dialog"
      aria-modal="true"
      aria-labelledby="rules-title"
    >
      <header class="mb-3 flex items-start justify-between gap-2">
        <h2 id="rules-title" class="text-lg font-semibold">{m.modal_rules_title()}</h2>
        <button
          type="button"
          class="text-fg-muted hover:text-fg flex h-9 w-9 items-center justify-center rounded-md"
          aria-label={m.modal_close()}
          onclick={onClose}
        >
          ×
        </button>
      </header>

      <p class="mb-3 text-sm">{m.modal_rules_intro()}</p>
      <ol class="mb-4 list-decimal space-y-2 pl-5 text-sm">
        <li>{m.modal_rules_step_1()}</li>
        <li>{m.modal_rules_step_2()}</li>
        <li>{m.modal_rules_step_3()}</li>
      </ol>

      <label class="text-fg-subtle flex items-center gap-2 text-sm">
        <input type="checkbox" class="size-4" bind:checked={hide} />
        {m.modal_rules_dont_show()}
      </label>
    </div>
  </div>
{/if}

<style>
  button {
    min-height: 44px;
    min-width: 44px;
  }
</style>
