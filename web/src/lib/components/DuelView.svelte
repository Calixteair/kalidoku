<script lang="ts">
  import Crown from "lucide-svelte/icons/crown";
  import Swords from "lucide-svelte/icons/swords";
  import * as m from "../../paraglide/messages.js";
  import { api, ApiError } from "../api/client.js";
  import Grid from "./Grid.svelte";

  // Inline structural type — the openapi-typescript output uses anonymous
  // response shapes for the duel endpoint, so importing a named alias would
  // overshoot. The fields match contracts/openapi.yaml /api/duels/{duelId}.
  type PlayerSummary = {
    pseudo: string;
    score: number;
    originalityScore: number;
    solved: number;
    finishedAt?: string | null;
    status: "active" | "won" | "lost" | "abandoned";
  };

  interface Props {
    domain: string;
  }

  let { domain }: Props = $props();

  let duelId = $state<string | null>(null);
  let sig = $state<string | null>(null);
  let gridId = $state<string | null>(null);
  let expiresAt = $state<string | null>(null);
  let players = $state<PlayerSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let playing = $state(false);

  const parseUrl = (): void => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    duelId = params.get("id");
    sig = params.get("sig");
  };

  const fetchDuel = async (): Promise<void> => {
    if (!duelId || !sig) {
      error = m.duel_missing_link();
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      const r = await api.get("/api/duels/{duelId}", { duelId }, { sig });
      gridId = r.gridId;
      expiresAt = r.expiresAt;
      players = r.players;
    } catch (err) {
      if (err instanceof ApiError) {
        if (err.status === 401) error = m.duel_bad_signature();
        else if (err.status === 404) error = m.duel_not_found();
        else if (err.status === 410) error = m.duel_expired();
        else error = m.error_network();
      } else {
        error = m.error_network();
      }
    } finally {
      loading = false;
    }
  };

  $effect(() => {
    parseUrl();
    void fetchDuel();
  });

  const formatExpires = (iso: string): string => {
    const d = new Date(iso);
    const locale = document.documentElement.lang === "en" ? "en-GB" : "fr-FR";
    return d.toLocaleDateString(locale, { day: "numeric", month: "long", year: "numeric" });
  };

  type IconLike = typeof Crown;
  const rankIcon = (idx: number): IconLike | null => (idx === 0 ? Crown : null);
</script>

<section class="flex flex-col gap-5">
  {#if loading}
    <div class="surface flex flex-col items-center justify-center gap-2 rounded-xl px-6 py-10">
      <span class="kd-spinner" aria-hidden="true"></span>
      <p class="text-fg-muted text-sm">{m.loading_stations()}</p>
    </div>
  {:else if error}
    <div class="surface flex flex-col gap-2 rounded-xl px-6 py-8">
      <p class="text-danger text-sm font-medium" role="alert">{error}</p>
      <a href="/" class="text-fg-subtle hover:text-fg text-sm underline">{m.back_to_home()}</a>
    </div>
  {:else if !playing}
    <header class="flex flex-col gap-2">
      <p class="eyebrow text-accent inline-flex items-center gap-1.5">
        <Swords size={12} aria-hidden="true" />
        {m.duel_eyebrow()}
      </p>
      <h1 class="font-display text-fg text-3xl font-semibold leading-tight">{m.duel_title()}</h1>
      <p class="text-fg-muted text-sm">{m.duel_subtitle()}</p>
      {#if expiresAt}
        <p class="text-fg-muted text-xs">
          {m.duel_expires_at({ date: formatExpires(expiresAt) })}
        </p>
      {/if}
    </header>

    <!-- Players that have already attempted the grid. Empty on a fresh
         duel — the friend who clicks first sees only the CTA below. -->
    {#if players.length > 0}
      <section class="surface overflow-hidden rounded-xl">
        <header
          class="bg-bg-subtle border-border flex items-center justify-between border-b px-4 py-2.5"
        >
          <p class="eyebrow text-fg-muted">{m.duel_players_heading()}</p>
          <p class="text-fg-muted text-xs tabular-nums">{players.length}</p>
        </header>
        <ol class="divide-border divide-y">
          {#each players as p, idx (idx)}
            {@const Icon = rankIcon(idx)}
            <li class="flex items-center gap-3 px-4 py-3">
              <span class="kd-duel-rank">
                {#if Icon}
                  <Icon size={14} aria-hidden="true" />
                {/if}
                <span class="tabular-nums">#{idx + 1}</span>
              </span>
              <span class="flex min-w-0 flex-1 flex-col">
                <span class="text-fg truncate text-sm font-medium">{p.pseudo}</span>
                <span class="text-fg-muted text-[11px]"
                  >{m.originality_label()} · {p.originalityScore}</span
                >
              </span>
              <span class="font-display tabular-nums text-fg text-base">{p.score}</span>
            </li>
          {/each}
        </ol>
      </section>
    {:else}
      <div class="surface rounded-xl px-4 py-3">
        <p class="text-fg-muted text-sm">{m.duel_no_players_yet()}</p>
      </div>
    {/if}

    <button
      type="button"
      class="kd-cta inline-flex items-center justify-center gap-2 self-start rounded-lg px-4 py-2.5 text-sm font-semibold"
      onclick={() => (playing = true)}
    >
      <Swords size={16} aria-hidden="true" />
      <span>{m.duel_play_button()}</span>
    </button>
  {:else if gridId}
    <Grid {domain} mode="duel" duelGridId={gridId} />
  {/if}
</section>

<style>
  .kd-spinner {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 3px solid color-mix(in oklab, var(--color-accent) 25%, transparent);
    border-top-color: var(--color-accent);
    animation: kd-spin 0.9s linear infinite;
  }
  @keyframes kd-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .kd-duel-rank {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    color: var(--color-fg-muted);
    font-size: 0.85rem;
    font-weight: 600;
    min-width: 2.25rem;
  }
  .kd-cta {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    border: 1px solid color-mix(in oklab, var(--color-accent) 70%, var(--color-fg));
    box-shadow: 0 1px 0 color-mix(in oklab, var(--color-accent) 50%, transparent) inset;
    transition:
      transform 120ms var(--ease-out),
      box-shadow 160ms var(--ease-out);
  }
  .kd-cta:hover {
    transform: translateY(-1px);
    box-shadow:
      0 1px 0 color-mix(in oklab, var(--color-accent) 50%, transparent) inset,
      0 6px 14px color-mix(in oklab, var(--color-accent) 30%, transparent);
  }
  .kd-cta:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
</style>
