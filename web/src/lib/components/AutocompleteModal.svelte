<script lang="ts">
  import Search from "lucide-svelte/icons/search";
  import X from "lucide-svelte/icons/x";
  import { onMount, untrack } from "svelte";
  import * as m from "../../paraglide/messages.js";
  import { api, ApiError } from "../api/client.js";
  import type { components } from "../api/types.js";
  import { highlight, normalize } from "../normalize.js";

  type Suggestion = components["schemas"]["AutocompleteResult"];

  interface Props {
    open: boolean;
    domain: string;
    cellLabel: string;
    candidatesCount?: number;
    onClose: () => void;
    onSubmit: (entity: { id: string; name: string }) => void | Promise<void>;
  }

  let { open, domain, cellLabel, candidatesCount, onClose, onSubmit }: Props = $props();

  let query = $state("");
  let suggestions = $state<Suggestion[]>([]);
  let activeIndex = $state(-1);
  let loading = $state(false);
  let errorMsg = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let abortCtrl: AbortController | null = null;
  let debounce: ReturnType<typeof setTimeout> | null = null;

  const listboxId = "autocomplete-listbox";
  const optionId = (i: number): string => `autocomplete-option-${i}`;

  const reset = (): void => {
    query = "";
    suggestions = [];
    activeIndex = -1;
    errorMsg = null;
  };

  // Don't surface suggestions for very short prefixes — otherwise the user
  // gets a hint after a single letter, which trivialises the puzzle.
  const MIN_QUERY_LEN = 3;

  const fetchSuggestions = async (q: string): Promise<void> => {
    if (q.trim().length < MIN_QUERY_LEN) {
      suggestions = [];
      activeIndex = -1;
      return;
    }
    if (abortCtrl) abortCtrl.abort();
    abortCtrl = new AbortController();
    loading = true;
    errorMsg = null;
    try {
      const data = await api.get(
        "/api/domains/{domain}/autocomplete",
        { domain },
        { q, limit: 8 },
        { signal: abortCtrl.signal },
      );
      // Replace atomically so the list never blinks empty between two
      // keystrokes — we keep showing the previous suggestions until the new
      // ones land, with a subtle loading overlay handling the in-flight
      // visual state.
      suggestions = data;
      activeIndex = data.length > 0 ? 0 : -1;
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") {
        // Superseded by a newer keystroke — leave existing state alone so the
        // list doesn't flash empty while the next request is on the wire.
        return;
      }
      if (err instanceof ApiError && err.code !== "network_error") {
        errorMsg = m.modal_autocomplete_error();
      } else {
        errorMsg = m.error_network();
      }
      suggestions = [];
      activeIndex = -1;
    } finally {
      loading = false;
    }
  };

  const onInput = (e: Event): void => {
    const v = (e.target as HTMLInputElement).value;
    query = v;
    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      void fetchSuggestions(v);
    }, 150);
  };

  const select = (s: Suggestion): void => {
    void onSubmit({ id: s.id, name: s.name });
  };

  const handleKey = (e: KeyboardEvent): void => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (suggestions.length === 0) return;
      activeIndex = (activeIndex + 1) % suggestions.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (suggestions.length === 0) return;
      activeIndex = (activeIndex - 1 + suggestions.length) % suggestions.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      const picked = suggestions[activeIndex];
      if (picked) select(picked);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Home") {
      activeIndex = 0;
    } else if (e.key === "End") {
      activeIndex = Math.max(0, suggestions.length - 1);
    }
  };

  const handleBackdrop = (e: MouseEvent): void => {
    if (e.target === e.currentTarget) onClose();
  };

  onMount(() => {
    if (open) inputEl?.focus();
  });

  $effect(() => {
    if (open) {
      // re-focus input each time it opens, reset state
      untrack(() => reset());
      queueMicrotask(() => inputEl?.focus());
    } else {
      if (debounce) clearTimeout(debounce);
      if (abortCtrl) abortCtrl.abort();
    }
  });

  const renderHighlighted = (name: string): Array<{ text: string; match: boolean }> => {
    return highlight(name, query);
  };

  const _ = $derived(normalize(query));

  const tooShort = $derived(query.trim().length > 0 && query.trim().length < MIN_QUERY_LEN);
  const empty = $derived(
    !loading && !errorMsg && query.trim().length >= MIN_QUERY_LEN && suggestions.length === 0,
  );
  // True while we have a list to show. Lets the markup keep rendering the
  // previous suggestions during an in-flight fetch instead of replacing them
  // with a spinner — the latter caused the inter-keystroke flicker.
  const hasResults = $derived(suggestions.length > 0);
</script>

{#if open}
  <div class="kd-modal-backdrop" role="presentation" onclick={handleBackdrop}>
    <div class="kd-modal" role="dialog" aria-modal="true" aria-labelledby="autocomplete-title">
      <header class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="eyebrow text-accent truncate">{cellLabel}</p>
          <h2
            id="autocomplete-title"
            class="font-display text-fg mt-0.5 text-xl font-semibold leading-tight"
          >
            {m.modal_autocomplete_title()}
          </h2>
          {#if typeof candidatesCount === "number"}
            <p class="text-fg-muted mt-1 text-xs">
              {candidatesCount === 1
                ? m.modal_autocomplete_candidates_count_one()
                : m.modal_autocomplete_candidates_count_many({ n: candidatesCount })}
            </p>
          {:else}
            <p class="text-fg-muted mt-1 text-xs">{m.modal_autocomplete_subtitle()}</p>
          {/if}
        </div>
        <button type="button" class="kd-modal-close" aria-label={m.modal_close()} onclick={onClose}>
          <X size={18} aria-hidden="true" />
        </button>
      </header>

      <div class="kd-input mt-4">
        <Search size={16} aria-hidden="true" class="kd-input__icon" />
        <label class="sr-only" for="autocomplete-input">{m.modal_autocomplete_placeholder()}</label>
        <input
          id="autocomplete-input"
          bind:this={inputEl}
          type="text"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          role="combobox"
          aria-controls={listboxId}
          aria-expanded={suggestions.length > 0}
          aria-activedescendant={activeIndex >= 0 ? optionId(activeIndex) : undefined}
          aria-autocomplete="list"
          placeholder={m.modal_autocomplete_placeholder()}
          value={query}
          oninput={onInput}
          onkeydown={handleKey}
        />
      </div>

      <div
        id={listboxId}
        role="listbox"
        aria-label={m.modal_autocomplete_title()}
        class="kd-list mt-3"
        class:kd-list--loading={loading}
      >
        {#if hasResults}
          <!-- Keep rendering the previous suggestions while a new fetch is in
               flight. The kd-list--loading class layers a subtle bar on top
               instead of swapping in a spinner that would jank the layout. -->
          {#each suggestions as s, i (s.id)}
            <button
              type="button"
              role="option"
              id={optionId(i)}
              aria-selected={i === activeIndex}
              class="kd-option"
              class:active={i === activeIndex}
              onclick={() => select(s)}
              onmouseenter={() => (activeIndex = i)}
            >
              <span class="kd-option__name text-fg">
                {#each renderHighlighted(s.name) as part}
                  {#if part.match}
                    <mark>{part.text}</mark>
                  {:else}
                    <span>{part.text}</span>
                  {/if}
                {/each}
              </span>
              {#if s.subtitle}
                <span class="kd-option__subtitle text-fg-muted">{s.subtitle}</span>
              {/if}
            </button>
          {/each}
        {:else if loading}
          <div class="kd-state">
            <span class="kd-state__spinner" aria-hidden="true"></span>
            <span class="text-fg-muted text-sm">{m.loading_stations()}</span>
          </div>
        {:else if errorMsg}
          <p class="text-danger px-1 py-2 text-sm" role="alert">{errorMsg}</p>
        {:else if tooShort}
          <p class="text-fg-muted px-1 py-2 text-sm">
            {m.modal_autocomplete_min_chars({ n: MIN_QUERY_LEN })}
          </p>
        {:else if empty}
          <div class="kd-empty">
            <p class="text-fg-subtle text-sm font-medium">{m.modal_autocomplete_no_results()}</p>
            <p class="text-fg-muted mt-1 text-xs">{m.modal_autocomplete_no_results_hint()}</p>
          </div>
        {/if}
      </div>

      <p class="text-fg-muted/80 mt-3 hidden text-[11px] tracking-wide sm:block">
        {m.modal_autocomplete_keyboard_hint()}
      </p>
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
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-xl);
    padding: 1.25rem;
    box-shadow: var(--shadow-modal);
    animation: kd-slide-up 220ms var(--ease-out);
  }
  @media (min-width: 640px) {
    .kd-modal {
      padding: 1.5rem;
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
  /* Search input with leading icon — kept high enough on mobile so the
     candidate count above remains readable while typing. */
  .kd-input {
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.kd-input__icon) {
    position: absolute;
    left: 12px;
    color: var(--color-fg-muted);
    pointer-events: none;
  }
  .kd-input input {
    width: 100%;
    height: 48px;
    padding: 0 12px 0 38px;
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    background: var(--color-bg-subtle);
    color: var(--color-fg);
    font-size: 16px;
    outline: none;
    transition:
      border-color 160ms var(--ease-out),
      box-shadow 160ms var(--ease-out),
      background-color 160ms var(--ease-out);
  }
  .kd-input input:focus {
    border-color: var(--color-accent);
    background: var(--color-bg-card);
    box-shadow: 0 0 0 3px var(--color-ring);
  }
  .kd-list {
    overflow-y: auto;
    max-height: 18rem;
    /* Reserve enough vertical space for ~3 options so the dialog doesn't
       jump in height between "empty hint", "loading" and "results" states.
       Above 3 options the list grows naturally up to max-height. */
    min-height: 9rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0 -0.25rem;
    padding: 0 0.25rem;
    position: relative;
  }
  /* Top progress bar that animates while a fetch is in flight. Sits on top
     of the (stale) suggestion list so the previous results stay visible —
     swapping them out for a spinner each keystroke was the source of the
     flicker we fixed here. */
  .kd-list--loading::before {
    content: "";
    position: sticky;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    margin-bottom: -2px;
    background: linear-gradient(90deg, transparent 0%, var(--color-accent) 50%, transparent 100%);
    background-size: 200% 100%;
    animation: kd-list-loading-shimmer 900ms linear infinite;
    z-index: 1;
    pointer-events: none;
  }
  @keyframes kd-list-loading-shimmer {
    from {
      background-position: 200% 0;
    }
    to {
      background-position: -200% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .kd-list--loading::before {
      animation: none;
      background: var(--color-accent);
      opacity: 0.6;
    }
  }
  .kd-state {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.25rem;
  }
  .kd-state__spinner {
    width: 14px;
    height: 14px;
    border-radius: 999px;
    border: 2px solid color-mix(in oklab, var(--color-fg) 18%, transparent);
    border-top-color: var(--color-accent);
    animation: kd-spin 700ms linear infinite;
  }
  .kd-empty {
    padding: 0.65rem 0.5rem;
  }
  .kd-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 0.65rem 0.65rem;
    border-radius: var(--radius-md);
    text-align: left;
    font-size: 0.875rem;
    line-height: 1.2;
    min-height: 44px;
    border: 1px solid transparent;
    transition:
      background-color 120ms var(--ease-out),
      border-color 120ms var(--ease-out);
  }
  .kd-option:hover {
    background: var(--color-bg-subtle);
  }
  .kd-option.active {
    background: color-mix(in oklab, var(--color-accent) 10%, transparent);
    border-color: color-mix(in oklab, var(--color-accent) 30%, transparent);
  }
  .kd-option__name {
    font-weight: 500;
  }
  .kd-option__name mark {
    background: color-mix(in oklab, var(--color-accent) 28%, transparent);
    color: var(--color-fg);
    padding: 0 1px;
    border-radius: 2px;
  }
  .kd-option__subtitle {
    font-size: 0.75rem;
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
  @keyframes kd-spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .kd-modal-backdrop,
    .kd-modal,
    .kd-state__spinner {
      animation: none;
    }
  }
</style>
