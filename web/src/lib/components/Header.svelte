<script lang="ts">
  import Archive from "lucide-svelte/icons/archive";
  import Home from "lucide-svelte/icons/home";
  import Trophy from "lucide-svelte/icons/trophy";
  import User from "lucide-svelte/icons/user";
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

  let pathname = $state<string>("/");
  $effect(() => {
    if (typeof window === "undefined") return;
    pathname = window.location.pathname.replace(/\/$/, "") || "/";
  });

  const isActive = (href: string): boolean => {
    const target = href.replace(/\/$/, "") || "/";
    if (target === "/") return pathname === "/";
    return pathname === target || pathname.startsWith(target + "/");
  };

  type NavItem = { href: string; label: string; icon: typeof Home };
  const items: NavItem[] = [
    { href: "/", label: m.nav_home(), icon: Home },
    { href: "/leaderboard", label: m.nav_leaderboard(), icon: Trophy },
    { href: "/archives", label: m.nav_archives(), icon: Archive },
    { href: "/profile", label: m.nav_profile(), icon: User },
  ];
</script>

<header
  class="border-border bg-bg/85 sticky top-0 z-40 border-b backdrop-blur-md backdrop-saturate-150 supports-[backdrop-filter]:bg-bg/70"
>
  <div class="mx-auto flex w-full max-w-screen-md items-center gap-3 px-4 py-3 md:px-6">
    <a
      href="/"
      class="group flex shrink-0 items-center gap-2"
      aria-label={m.site_title() + " — " + m.site_tagline()}
    >
      <span
        class="bg-accent text-accent-fg ring-accent/30 flex h-9 w-9 items-center justify-center rounded-lg font-display text-base font-semibold ring-1 transition-transform group-hover:-rotate-3"
        aria-hidden="true"
      >
        k
      </span>
      <span class="flex flex-col leading-tight">
        <span class="font-display text-fg text-lg font-semibold tracking-tight"
          >{m.site_title()}</span
        >
        <span class="text-fg-muted hidden text-[10px] uppercase tracking-[0.18em] sm:block"
          >daily grid</span
        >
      </span>
    </a>

    <nav aria-label="Primary" class="ml-auto flex items-center gap-1">
      <ul class="flex items-center gap-0.5 sm:gap-1">
        {#each items as item (item.href)}
          {@const Icon = item.icon}
          {@const active = isActive(item.href)}
          <li>
            <a
              href={item.href}
              class="nav-link"
              class:active
              aria-current={active ? "page" : undefined}
              title={item.label}
            >
              <Icon size={18} aria-hidden="true" />
              <span class="nav-label hidden md:inline">{item.label}</span>
            </a>
          </li>
        {/each}
      </ul>
      <span class="bg-border mx-1 hidden h-6 w-px sm:block" aria-hidden="true"></span>
      <ThemeToggle />
    </nav>
  </div>
</header>

<style>
  .nav-link {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.65rem;
    border-radius: var(--radius-md);
    color: var(--color-fg-muted);
    font-size: 0.875rem;
    font-weight: 500;
    min-height: 40px;
    transition:
      color 160ms var(--ease-out),
      background-color 160ms var(--ease-out);
    position: relative;
  }
  .nav-link:hover {
    color: var(--color-fg);
    background-color: color-mix(in oklab, var(--color-fg) 6%, transparent);
  }
  .nav-link.active {
    color: var(--color-fg);
    background-color: color-mix(in oklab, var(--color-accent) 12%, transparent);
  }
  .nav-link.active::after {
    content: "";
    position: absolute;
    inset: auto 0.65rem -2px 0.65rem;
    height: 2px;
    background: var(--color-accent);
    border-radius: 1px;
  }
</style>
