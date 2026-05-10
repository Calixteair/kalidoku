<script lang="ts">
  import * as m from "../../paraglide/messages.js";
  import { setLanguageTag, type AvailableLanguageTag } from "../../paraglide/runtime.js";

  if (typeof document !== "undefined") {
    const tag = document.documentElement.lang;
    if (tag === "fr" || tag === "en") {
      setLanguageTag(tag as AvailableLanguageTag);
    }
  }
</script>

<footer class="border-border bg-bg-card mt-auto border-t">
  <div
    class="text-fg-muted mx-auto flex max-w-screen-md flex-wrap items-center justify-between gap-2 px-4 py-3 text-xs"
  >
    <ul class="flex flex-wrap gap-3">
      <li><a class="hover:text-fg" href="/legal/privacy">{m.page_legal_privacy_title()}</a></li>
      <li><a class="hover:text-fg" href="/legal/cookies">{m.page_legal_cookies_title()}</a></li>
      <li><a class="hover:text-fg" href="/legal/terms">{m.page_legal_terms_title()}</a></li>
    </ul>
    <span>kalidoku · AGPL-3.0</span>
  </div>
</footer>
