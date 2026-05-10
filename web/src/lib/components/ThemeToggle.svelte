<script lang="ts">
  import Sun from "lucide-svelte/icons/sun";
  import Moon from "lucide-svelte/icons/moon";

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
</script>

<button
  type="button"
  class="text-fg-subtle hover:text-fg flex h-9 w-9 items-center justify-center rounded transition"
  aria-label={theme === "dark" ? "Activer le thème clair" : "Activer le thème sombre"}
  aria-pressed={theme === "dark"}
  title={theme === "dark" ? "Thème clair" : "Thème sombre"}
  onclick={toggle}
>
  {#if theme === "dark"}
    <Sun aria-hidden="true" size={18} />
  {:else}
    <Moon aria-hidden="true" size={18} />
  {/if}
</button>
