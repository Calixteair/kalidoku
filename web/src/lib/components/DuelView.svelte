<script lang="ts">
  import Check from "lucide-svelte/icons/check";
  import Copy from "lucide-svelte/icons/copy";
  import Crown from "lucide-svelte/icons/crown";
  import Share2 from "lucide-svelte/icons/share-2";
  import Sparkles from "lucide-svelte/icons/sparkles";
  import Swords from "lucide-svelte/icons/swords";
  import * as m from "../../paraglide/messages.js";
  import { api, ApiError } from "../api/client.js";
  import { copyToClipboard, shareNative } from "../share.js";
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

  /**
   * Three screens behind one route:
   * - `landing`: /duel hit without ?id= — show "create a fresh duel" CTA.
   * - `view`:    /duel?id=…&sig=… valid — show the duel summary + play CTA.
   * - `playing`: mounted Grid in duel mode.
   * Errors flip the screen to `landing` after showing a banner.
   */
  type Screen = "landing" | "view" | "playing";

  interface Props {
    domain: string;
  }

  let { domain }: Props = $props();

  let screen = $state<Screen>("landing");
  let gridId = $state<string | null>(null);
  let expiresAt = $state<string | null>(null);
  let players = $state<PlayerSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Share-link creation state for the landing screen.
  let creating = $state(false);
  let createError = $state<string | null>(null);
  let createdShareUrl = $state<string | null>(null);
  let createdCopied = $state(false);

  const parseUrl = (): { id: string | null; sig: string | null } => {
    if (typeof window === "undefined") return { id: null, sig: null };
    const params = new URLSearchParams(window.location.search);
    return { id: params.get("id"), sig: params.get("sig") };
  };

  const fetchDuel = async (id: string, s: string): Promise<void> => {
    loading = true;
    error = null;
    try {
      const r = await api.get("/api/duels/{duelId}", { duelId: id }, { sig: s });
      gridId = r.gridId;
      expiresAt = r.expiresAt;
      players = r.players;
      screen = "view";
    } catch (err) {
      if (err instanceof ApiError) {
        if (err.status === 401) error = m.duel_bad_signature();
        else if (err.status === 404) error = m.duel_not_found();
        else if (err.status === 410) error = m.duel_expired();
        else error = m.error_network();
      } else {
        error = m.error_network();
      }
      // Fall back to the landing screen so the player can still create a
      // fresh duel — surfacing only the error leaves them stuck.
      screen = "landing";
    } finally {
      loading = false;
    }
  };

  $effect(() => {
    const { id, sig: s } = parseUrl();
    if (id && s) {
      void fetchDuel(id, s);
    } else {
      // No query params → landing screen, no fetch.
      loading = false;
      screen = "landing";
    }
  });

  const createDuel = async (): Promise<void> => {
    if (creating) return;
    creating = true;
    createError = null;
    createdShareUrl = null;
    createdCopied = false;
    try {
      const r = await api.post("/api/duels", undefined, { domain });
      createdShareUrl = r.shareUrl;
      const shared = await shareNative(r.shareUrl, m.duel_share_title());
      if (!shared) {
        const ok = await copyToClipboard(r.shareUrl);
        createdCopied = ok;
        if (ok) setTimeout(() => (createdCopied = false), 2400);
      }
    } catch (err) {
      createError = err instanceof ApiError ? err.message : m.error_network();
    } finally {
      creating = false;
    }
  };

  const copyCreated = async (): Promise<void> => {
    if (!createdShareUrl) return;
    const ok = await copyToClipboard(createdShareUrl);
    createdCopied = ok;
    if (ok) setTimeout(() => (createdCopied = false), 2400);
  };

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
  {:else if screen === "landing"}
    <!-- Landing: invite the player to create a fresh duel link. Reached both
         when /duel is visited without query params and as a fallback when an
         existing duel link errors out (404 / 410 / 401). -->
    <header class="flex flex-col gap-2">
      <p class="eyebrow text-accent inline-flex items-center gap-1.5">
        <Sparkles size={12} aria-hidden="true" />
        {m.duel_create_eyebrow()}
      </p>
      <h1 class="font-display text-fg text-3xl font-semibold leading-tight">
        {m.duel_create_title()}
      </h1>
      <p class="text-fg-muted text-sm">{m.duel_create_subtitle()}</p>
    </header>

    {#if error}
      <p class="text-danger text-sm" role="alert">{error}</p>
    {/if}

    {#if createdShareUrl === null}
      <button
        type="button"
        class="kd-cta inline-flex items-center justify-center gap-2 self-start rounded-lg px-4 py-2.5 text-sm font-semibold"
        disabled={creating}
        onclick={createDuel}
      >
        <Swords size={16} aria-hidden="true" />
        <span>{creating ? m.duel_share_creating() : m.duel_create_button()}</span>
      </button>
    {:else}
      <div class="border-border bg-bg-subtle flex flex-col gap-2 rounded-xl border p-4">
        <p class="eyebrow text-fg-muted">{m.duel_share_ready()}</p>
        <p class="text-fg break-all text-xs font-mono">{createdShareUrl}</p>
        <div class="flex flex-col gap-2 sm:flex-row">
          <button
            type="button"
            class="kd-cta inline-flex flex-1 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold"
            onclick={() => void shareNative(createdShareUrl, m.duel_share_title())}
          >
            <Share2 size={14} aria-hidden="true" />
            <span>{m.share_button()}</span>
          </button>
          <button
            type="button"
            class="kd-secondary inline-flex flex-1 items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold"
            onclick={copyCreated}
          >
            {#if createdCopied}
              <Check size={14} aria-hidden="true" />
              <span>{m.share_copied()}</span>
            {:else}
              <Copy size={14} aria-hidden="true" />
              <span>{m.copy_result()}</span>
            {/if}
          </button>
        </div>
        <p class="text-fg-muted mt-1 text-[11px]">{m.duel_create_share_hint()}</p>
      </div>
    {/if}
    {#if createError}
      <p class="text-danger text-sm" role="alert">{createError}</p>
    {/if}
  {:else if screen === "view"}
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
      onclick={() => (screen = "playing")}
    >
      <Swords size={16} aria-hidden="true" />
      <span>{m.duel_play_button()}</span>
    </button>
  {:else if screen === "playing" && gridId}
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
  .kd-cta:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow:
      0 1px 0 color-mix(in oklab, var(--color-accent) 50%, transparent) inset,
      0 6px 14px color-mix(in oklab, var(--color-accent) 30%, transparent);
  }
  .kd-cta:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .kd-cta:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
  .kd-secondary {
    background: transparent;
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    transition: background-color 140ms var(--ease-out);
  }
  .kd-secondary:hover {
    background: color-mix(in oklab, var(--color-fg) 6%, transparent);
  }
  .kd-secondary:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
</style>
