<script lang="ts">
  import Moon from "lucide-svelte/icons/moon";
  import Sun from "lucide-svelte/icons/sun";
  import * as m from "../../paraglide/messages.js";

  // Two-state toggle: light (default) and dark. Persisted in localStorage.
  // The inline boot script in Base.astro applies the class before paint to avoid FOUC.
  const STORAGE_KEY = "kalidoku.theme";
  type Theme = "light" | "dark";

  let theme = $state<Theme>("light");

  $effect(() => {
    if (typeof document === "undefined") return;
    theme = document.documentElement.classList.contains("dark") ? "dark" : "light";
  });

  function applyTheme(t: Theme) {
    if (t === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
    try {
      localStorage.setItem(STORAGE_KEY, t);
    } catch {
      /* private mode etc. */
    }
  }

  function toggle() {
    const next: Theme = theme === "dark" ? "light" : "dark";
    theme = next;
    applyTheme(next);
  }

  const labelDark = m.theme_switch_to_dark();
  const labelLight = m.theme_switch_to_light();
</script>

<button
  type="button"
  class="theme-btn ring-border hover:ring-fg/30 inline-flex h-9 w-9 items-center justify-center rounded-md ring-1 transition-colors"
  aria-label={theme === "dark" ? labelLight : labelDark}
  aria-pressed={theme === "dark"}
  title={theme === "dark" ? labelLight : labelDark}
  onclick={toggle}
>
  <span class="icon-wrap" aria-hidden="true">
    {#if theme === "dark"}
      <Sun size={18} />
    {:else}
      <Moon size={18} />
    {/if}
  </span>
</button>

<style>
  .theme-btn {
    color: var(--color-fg-muted);
    background: transparent;
  }
  .theme-btn:hover {
    color: var(--color-fg);
  }
  .icon-wrap {
    display: inline-flex;
    transform-origin: center;
    animation: theme-rotate 220ms var(--ease-out);
  }
  @keyframes theme-rotate {
    from {
      transform: rotate(-45deg) scale(0.85);
      opacity: 0;
    }
    to {
      transform: rotate(0deg) scale(1);
      opacity: 1;
    }
  }
</style>
