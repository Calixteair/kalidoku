<script lang="ts">
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

<label class="text-fg-subtle inline-flex items-center gap-2 text-sm">
  <span class="sr-only">{m.language_switch()}</span>
  <select
    class="border-border bg-bg-card rounded-md border px-2 py-1 text-sm"
    value={current}
    onchange={onChange}
    aria-label={m.language_switch()}
  >
    {#each availableLanguageTags as tag (tag)}
      <option value={tag}>{labels[tag]}</option>
    {/each}
  </select>
</label>
