<script lang="ts">
  import Archive from "lucide-svelte/icons/archive";
  import ChevronDown from "lucide-svelte/icons/chevron-down";
  import Dices from "lucide-svelte/icons/dices";
  import Home from "lucide-svelte/icons/home";
  import Trophy from "lucide-svelte/icons/trophy";
  import User from "lucide-svelte/icons/user";
  import * as m from "../../paraglide/messages.js";
  import { setLanguageTag, type AvailableLanguageTag } from "../../paraglide/runtime.js";
  import { DEFAULT_DOMAIN_ID, DOMAINS, findDomain } from "../domains.js";
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

  // currentDomain: first URL segment if it matches a known domain id; else
  // the default. Drives both the active-link highlight and the link targets
  // — so /paris-metro/leaderboard's 'Solo' link points back to
  // /paris-metro/play and not to /rer/play.
  const currentDomain = $derived.by((): string => {
    if (pathname === "/") return DEFAULT_DOMAIN_ID;
    const seg = pathname.split("/").filter(Boolean)[0] ?? "";
    return findDomain(seg) ? seg : DEFAULT_DOMAIN_ID;
  });
  const currentEntry = $derived(findDomain(currentDomain));

  const isActive = (href: string): boolean => {
    const target = href.replace(/\/$/, "") || "/";
    if (target === "/") return pathname === "/";
    return pathname === target || pathname.startsWith(target + "/");
  };

  // The hub page sits at "/" — link "Home" there so the player always has a
  // one-click escape back to domain selection.
  type NavItem = { href: string; label: string; icon: typeof Home };
  const items = $derived.by((): NavItem[] => [
    { href: `/${currentDomain}/`, label: m.nav_home(), icon: Home },
    { href: `/${currentDomain}/play`, label: m.nav_solo(), icon: Dices },
    { href: `/${currentDomain}/leaderboard`, label: m.nav_leaderboard(), icon: Trophy },
    { href: `/${currentDomain}/archives`, label: m.nav_archives(), icon: Archive },
    { href: "/profile", label: m.nav_profile(), icon: User },
  ]);

  let domainMenuOpen = $state(false);
  const toggleDomainMenu = (): void => {
    domainMenuOpen = !domainMenuOpen;
  };
  const handleDocClick = (e: MouseEvent): void => {
    const target = e.target as Element | null;
    if (!target || !target.closest(".kd-domain-pill")) {
      domainMenuOpen = false;
    }
  };
  $effect(() => {
    if (typeof document === "undefined") return;
    if (!domainMenuOpen) return;
    document.addEventListener("click", handleDocClick);
    return () => document.removeEventListener("click", handleDocClick);
  });

  // Same path on the destination domain, minus the leading /<currentDomain>.
  // Lets the user hop between domains while staying on the same view
  // (e.g. /paris-metro/leaderboard → /rer/leaderboard on switch).
  const sameViewOn = (targetDomain: string): string => {
    if (pathname === "/") return `/${targetDomain}/`;
    const parts = pathname.split("/").filter(Boolean);
    if (parts.length === 0) return `/${targetDomain}/`;
    // Replace the leading segment if it's a known domain, otherwise prefix.
    if (findDomain(parts[0])) {
      parts[0] = targetDomain;
    } else {
      parts.unshift(targetDomain);
    }
    return "/" + parts.join("/");
  };

  // Domain picker visible on every page except /profile, /duel and /legal/*.
  // It's a navigation aid that wouldn't make sense on global pages.
  const showDomainPicker = $derived(
    pathname !== "/profile" && !pathname.startsWith("/legal") && !pathname.startsWith("/duel"),
  );
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

    {#if showDomainPicker && currentEntry}
      <!-- Domain pill: shows the active domain, opens a dropdown to switch.
           Stays compact on mobile (the abbreviated id, e.g. "Métro" / "RER")
           and grows to the full label on >= sm. -->
      <div class="kd-domain-pill relative">
        <button
          type="button"
          class="kd-domain-trigger"
          onclick={toggleDomainMenu}
          aria-haspopup="listbox"
          aria-expanded={domainMenuOpen}
          aria-label={m.nav_domain_picker_label()}
        >
          <span class="kd-domain-trigger__label">{currentEntry.nameFr}</span>
          <ChevronDown size={14} aria-hidden="true" class="kd-domain-trigger__chev" />
        </button>
        {#if domainMenuOpen}
          <ul class="kd-domain-menu" role="listbox" aria-label={m.nav_domain_picker_label()}>
            {#each DOMAINS as d (d.id)}
              {@const isCurrent = d.id === currentDomain}
              <li role="presentation">
                <a
                  role="option"
                  href={sameViewOn(d.id)}
                  class="kd-domain-menu__item"
                  class:active={isCurrent}
                  aria-selected={isCurrent}
                  onclick={() => (domainMenuOpen = false)}
                >
                  {d.nameFr}
                </a>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
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

  /* Domain picker pill. Sits between the logo and the nav; hidden on the
     pages where domain has no meaning (profile, legal, duel). */
  .kd-domain-trigger {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.7rem;
    height: 32px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-bg-subtle);
    color: var(--color-fg);
    font-size: 0.78rem;
    font-weight: 600;
    max-width: 14ch;
    transition:
      border-color 140ms var(--ease-out),
      background-color 140ms var(--ease-out);
  }
  .kd-domain-trigger:hover {
    border-color: color-mix(in oklab, var(--color-accent) 40%, var(--color-border));
  }
  .kd-domain-trigger:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
  .kd-domain-trigger__label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.kd-domain-trigger__chev) {
    flex: 0 0 auto;
    color: var(--color-fg-muted);
  }
  .kd-domain-menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    min-width: 14rem;
    margin: 0;
    padding: 0.3rem;
    list-style: none;
    background: var(--color-bg-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal);
    z-index: 30;
    animation: kd-fade-in 140ms var(--ease-out);
  }
  @keyframes kd-fade-in {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .kd-domain-menu__item {
    display: block;
    padding: 0.5rem 0.7rem;
    border-radius: var(--radius-md);
    color: var(--color-fg);
    font-size: 0.85rem;
    font-weight: 500;
    text-decoration: none;
    transition: background-color 140ms var(--ease-out);
  }
  .kd-domain-menu__item:hover {
    background: color-mix(in oklab, var(--color-fg) 6%, transparent);
  }
  .kd-domain-menu__item.active {
    color: var(--color-accent);
    background: color-mix(in oklab, var(--color-accent) 12%, transparent);
  }
</style>
