<script lang="ts">
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
      suggestions = data;
      activeIndex = data.length > 0 ? 0 : -1;
    } catch (err) {
      if (err instanceof ApiError && err.code !== "network_error") {
        errorMsg = m.modal_autocomplete_error();
      } else if (err instanceof DOMException && err.name === "AbortError") {
        return;
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
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-black/40 p-2 sm:p-4"
    role="presentation"
    onclick={handleBackdrop}
  >
    <div
      class="bg-bg-card border-border flex max-h-[90vh] w-full max-w-md flex-col rounded-lg border p-3 shadow-xl sm:p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="autocomplete-title"
    >
      <header class="mb-2 flex items-start justify-between gap-2">
        <div>
          <h2 id="autocomplete-title" class="text-base font-semibold">
            {m.modal_autocomplete_title()}
          </h2>
          <p class="text-fg-muted text-xs">
            {cellLabel}{#if typeof candidatesCount === "number"}
              · {candidatesCount === 1
                ? m.modal_autocomplete_candidates_count_one()
                : m.modal_autocomplete_candidates_count_many({ n: candidatesCount })}{/if}
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
        class="border-border bg-bg focus:border-accent w-full rounded-md border px-3 py-2 outline-none"
      />

      <div
        id={listboxId}
        role="listbox"
        aria-label={m.modal_autocomplete_title()}
        class="mt-2 flex max-h-72 flex-col overflow-y-auto"
      >
        {#if loading}
          <p class="text-fg-muted px-2 py-2 text-sm">{m.loading_stations()}</p>
        {:else if errorMsg}
          <p class="text-danger px-2 py-2 text-sm" role="alert">{errorMsg}</p>
        {:else if query.trim().length > 0 && query.trim().length < MIN_QUERY_LEN}
          <p class="text-fg-muted px-2 py-2 text-sm">
            {m.modal_autocomplete_min_chars({ n: MIN_QUERY_LEN })}
          </p>
        {:else if query.trim().length >= MIN_QUERY_LEN && suggestions.length === 0}
          <p class="text-fg-muted px-2 py-2 text-sm">{m.modal_autocomplete_no_results()}</p>
        {:else}
          {#each suggestions as s, i (s.id)}
            <button
              type="button"
              role="option"
              id={optionId(i)}
              aria-selected={i === activeIndex}
              class="hover:bg-bg-subtle flex flex-col items-start gap-0 px-3 py-2 text-left text-sm"
              class:active={i === activeIndex}
              onclick={() => select(s)}
              onmouseenter={() => (activeIndex = i)}
            >
              <span class="text-fg">
                {#each renderHighlighted(s.name) as part}
                  {#if part.match}<mark class="bg-warning/30 text-fg rounded px-0.5"
                      >{part.text}</mark
                    >{:else}<span>{part.text}</span>{/if}
                {/each}
              </span>
              {#if s.subtitle}
                <span class="text-fg-muted text-xs">{s.subtitle}</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  button[role="option"] {
    min-height: 44px;
  }
  .active {
    background: var(--color-bg-subtle);
    outline: 2px solid var(--color-accent);
    outline-offset: -2px;
  }
</style>
