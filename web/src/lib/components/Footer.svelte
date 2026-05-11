<script lang="ts">
  import Github from "lucide-svelte/icons/github";
  import * as m from "../../paraglide/messages.js";
  import { setLanguageTag, type AvailableLanguageTag } from "../../paraglide/runtime.js";
  import LangSwitcher from "./LangSwitcher.svelte";

  if (typeof document !== "undefined") {
    const tag = document.documentElement.lang;
    if (tag === "fr" || tag === "en") {
      setLanguageTag(tag as AvailableLanguageTag);
    }
  }
</script>

<footer class="border-border mt-auto border-t">
  <div
    class="mx-auto flex w-full max-w-screen-md flex-col gap-4 px-4 py-6 md:flex-row md:items-center md:justify-between md:px-6"
  >
    <div class="flex flex-col gap-1">
      <span class="font-display text-fg text-base font-semibold tracking-tight"
        >{m.site_title()}</span
      >
      <span class="text-fg-muted text-xs">AGPL-3.0 · {m.site_tagline()}</span>
    </div>

    <nav aria-label="Legal" class="flex flex-wrap items-center gap-x-4 gap-y-2 text-xs">
      <ul class="text-fg-muted flex flex-wrap items-center gap-x-3 gap-y-1">
        <li>
          <a class="hover:text-fg transition-colors" href="/legal/privacy"
            >{m.page_legal_privacy_title()}</a
          >
        </li>
        <li aria-hidden="true" class="text-fg-muted/40">·</li>
        <li>
          <a class="hover:text-fg transition-colors" href="/legal/cookies"
            >{m.page_legal_cookies_title()}</a
          >
        </li>
        <li aria-hidden="true" class="text-fg-muted/40">·</li>
        <li>
          <a class="hover:text-fg transition-colors" href="/legal/terms"
            >{m.page_legal_terms_title()}</a
          >
        </li>
      </ul>
    </nav>

    <div class="flex items-center gap-2">
      <LangSwitcher />
      <a
        href="https://github.com/Calixteair/kalidoku"
        target="_blank"
        rel="noopener"
        class="text-fg-muted hover:text-fg ring-border hover:ring-fg/30 inline-flex h-9 w-9 items-center justify-center rounded-md ring-1 transition-colors"
        aria-label="GitHub"
        title="GitHub"
      >
        <Github size={16} aria-hidden="true" />
      </a>
    </div>
  </div>
</footer>
