<script lang="ts">
  import * as m from "../../paraglide/messages.js";
  import { setLanguageTag, type AvailableLanguageTag } from "../../paraglide/runtime.js";
  import ThemeToggle from "./ThemeToggle.svelte";

  // Re-apply the locale that the inline boot script wrote to <html lang>, so the
  // paraglide messages below resolve to the user's choice instead of the build
  // default. Without this Astro's static HTML stays in fr even when ?lang=en.
  if (typeof document !== "undefined") {
    const tag = document.documentElement.lang;
    if (tag === "fr" || tag === "en") {
      setLanguageTag(tag as AvailableLanguageTag);
    }
  }
</script>

<header class="border-border bg-bg-card border-b">
  <div class="mx-auto flex max-w-screen-md items-center justify-between gap-2 px-4 py-3">
    <a href="/" class="text-fg text-base font-bold tracking-tight">{m.site_title()}</a>
    <nav aria-label="Primary" class="flex items-center gap-3">
      <ul class="flex items-center gap-3 text-sm">
        <li><a class="text-fg-subtle hover:text-fg" href="/archives">{m.nav_archives()}</a></li>
        <li>
          <a class="text-fg-subtle hover:text-fg" href="/leaderboard">{m.nav_leaderboard()}</a>
        </li>
        <li><a class="text-fg-subtle hover:text-fg" href="/profile">{m.nav_profile()}</a></li>
      </ul>
      <ThemeToggle />
    </nav>
  </div>
</header>
