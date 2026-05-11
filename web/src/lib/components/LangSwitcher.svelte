<script lang="ts">
  import Globe from "lucide-svelte/icons/globe";
  import * as m from "../../paraglide/messages.js";
  import { availableLanguageTags, languageTag } from "../../paraglide/runtime.js";
  import { setLocale, type Locale } from "../i18n.js";

  let current = $state<Locale>(languageTag());

  const labels: Record<Locale, string> = {
    fr: m.language_fr(),
    en: m.language_en(),
  };

  const onChange = (e: Event): void => {
    const v = (e.target as HTMLSelectElement).value;
    if (v === "fr" || v === "en") {
      setLocale(v);
      current = v;
      // Reload to re-render Astro pages with the new locale.
      if (typeof window !== "undefined") {
        const url = new URL(window.location.href);
        url.searchParams.set("lang", v);
        window.location.assign(url.toString());
      }
    }
  };
</script>

<label
  class="text-fg-muted ring-border hover:ring-fg/30 inline-flex h-9 items-center gap-2 rounded-md ring-1 transition-colors"
>
  <span class="sr-only">{m.language_switch()}</span>
  <Globe size={14} aria-hidden="true" class="ml-2.5" />
  <select
    class="lang-select text-fg appearance-none bg-transparent pr-3 text-sm font-medium outline-none"
    value={current}
    onchange={onChange}
    aria-label={m.language_switch()}
  >
    {#each availableLanguageTags as tag (tag)}
      <option value={tag}>{labels[tag]}</option>
    {/each}
  </select>
</label>

<style>
  .lang-select {
    background-image: none;
    padding: 0.4rem 0.25rem 0.4rem 0;
  }
  /* Forms plugin gives selects a default chevron — kill it, the Globe icon is enough */
  .lang-select::-ms-expand {
    display: none;
  }
</style>
