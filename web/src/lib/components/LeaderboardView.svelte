<script lang="ts">
  import ArrowDown from "lucide-svelte/icons/arrow-down";
  import Crown from "lucide-svelte/icons/crown";
  import Medal from "lucide-svelte/icons/medal";
  import Trophy from "lucide-svelte/icons/trophy";
  import { api, ApiError } from "../api/client.js";
  import type { components } from "../api/types.js";
  import * as m from "../../paraglide/messages.js";

  type LeaderboardPage = components["schemas"]["LeaderboardPage"];
  type Period = NonNullable<LeaderboardPage["period"]>;
  type SortKey = NonNullable<LeaderboardPage["sort"]>;

  interface Props {
    domain: string;
  }

  let { domain }: Props = $props();

  let page = $state<LeaderboardPage | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let period = $state<Period>("daily");
  let sort = $state<SortKey>("score");

  const load = async (): Promise<void> => {
    loading = true;
    error = null;
    try {
      page = await api.get("/api/leaderboard/{domain}", { domain }, { period, sort });
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        page = { items: [], period, sort };
      } else {
        error = m.error_network();
      }
    } finally {
      loading = false;
    }
  };

  $effect(() => {
    // Re-fetch on tab / sort change. period + sort live in the closure so
    // the runtime tracks them as deps automatically.
    void period;
    void sort;
    void load();
  });

  type RankIcon = typeof Crown;
  const rankIcon = (rank: number): RankIcon | null => {
    if (rank === 1) return Crown;
    if (rank === 2) return Trophy;
    if (rank === 3) return Medal;
    return null;
  };

  const periods: { value: Period; label: () => string }[] = [
    { value: "daily", label: m.leaderboard_period_daily },
    { value: "weekly", label: m.leaderboard_period_weekly },
    { value: "all-time", label: m.leaderboard_period_all_time },
  ];

  /** Sort keys clickable as column headers. `time` and `count` only make
   * sense on weekly / all-time — they're greyed out (still sortable) on
   * daily but the resulting row is the same single game so the data ranks
   * the same as `score`. We don't disable them — they're consistent. */
  const headers: {
    key: SortKey;
    label: () => string;
    align: "left" | "right";
    desktopOnly: boolean;
  }[] = [
    { key: "wins", label: m.leaderboard_col_wins, align: "right", desktopOnly: true },
    { key: "count", label: m.leaderboard_col_count, align: "right", desktopOnly: true },
    { key: "time", label: m.leaderboard_col_time, align: "right", desktopOnly: true },
    {
      key: "originality",
      label: m.leaderboard_col_originality,
      align: "right",
      desktopOnly: true,
    },
    {
      key: "score",
      label: m.leaderboard_col_score_short,
      align: "right",
      desktopOnly: false,
    },
  ];

  const formatTime = (sec: number | null | undefined): string => {
    if (sec === null || sec === undefined) return "—";
    if (sec < 60) return `${sec}s`;
    const m_ = Math.floor(sec / 60);
    const s = sec % 60;
    return `${m_}m${s.toString().padStart(2, "0")}`;
  };
</script>

<section class="kd-leaderboard flex flex-col gap-3">
  <!-- Period tabs — kept on a single row at 360px by truncating the labels
       in CSS rather than abbreviating the i18n strings. -->
  <nav class="kd-tabs" aria-label={m.leaderboard_period_label()}>
    {#each periods as p (p.value)}
      <button
        type="button"
        class="kd-tab"
        class:active={period === p.value}
        onclick={() => (period = p.value)}
        aria-pressed={period === p.value}
      >
        {p.label()}
      </button>
    {/each}
  </nav>

  {#if loading}
    <ul class="border-border surface divide-border divide-y overflow-hidden rounded-xl">
      {#each [1, 2, 3, 4, 5] as i (i)}
        <li class="flex items-center gap-3 px-4 py-3">
          <span class="kd-skel kd-skel--rank" aria-hidden="true"></span>
          <span class="kd-skel kd-skel--name flex-1" aria-hidden="true"></span>
          <span class="kd-skel kd-skel--score" aria-hidden="true"></span>
        </li>
      {/each}
    </ul>
    <p class="sr-only">{m.loading_stations()}</p>
  {:else if error}
    <p class="text-danger text-sm" role="alert">{error}</p>
  {:else if !page || page.items.length === 0}
    <div class="surface flex flex-col items-center gap-2 rounded-xl px-6 py-10 text-center">
      <Trophy size={28} aria-hidden="true" class="text-fg-muted" />
      <p class="text-fg-subtle text-sm">{m.page_leaderboard_empty()}</p>
    </div>
  {:else}
    <ol class="surface divide-border overflow-hidden rounded-xl">
      <li class="kd-head bg-bg-subtle items-center gap-3 px-4 py-2.5 border-b border-border">
        <span class="eyebrow text-fg-muted">{m.page_leaderboard_rank_col()}</span>
        <span class="eyebrow text-fg-muted">{m.page_leaderboard_player_col()}</span>
        {#each headers as h (h.key)}
          <button
            type="button"
            class="kd-head__sort eyebrow"
            class:active={sort === h.key}
            class:kd-head__sort--right={h.align === "right"}
            class:kd-head__sort--desktop={h.desktopOnly}
            onclick={() => (sort = h.key)}
            aria-pressed={sort === h.key}
            aria-label={h.label()}
          >
            <span class="truncate">{h.label()}</span>
            {#if sort === h.key}
              <ArrowDown size={10} aria-hidden="true" />
            {/if}
          </button>
        {/each}
      </li>
      {#each page.items as entry (entry.profile.id + "-" + entry.rank)}
        {@const Icon = rankIcon(entry.rank)}
        <li
          class="kd-row items-center gap-3 px-4 py-3"
          class:kd-row--top1={entry.rank === 1}
          class:kd-row--top2={entry.rank === 2}
          class:kd-row--top3={entry.rank === 3}
        >
          <span class="kd-rank font-display">
            {#if Icon}
              <Icon size={16} aria-hidden="true" />
            {/if}
            <span class="tabular-nums">#{entry.rank}</span>
          </span>
          <span class="flex min-w-0 flex-col gap-0.5">
            <span class="text-fg truncate text-sm font-medium">{entry.profile.pseudo}</span>
            <!-- Mobile secondary line: surfaces the metrics hidden on small
                 screens so the player still sees what their score is made of. -->
            <span class="kd-row__sub sm:hidden" aria-hidden="true">
              {#if period === "daily"}
                {m.leaderboard_col_originality()} · {entry.bestOriginality}
              {:else}
                {entry.gamesCount}
                {m.leaderboard_col_count_unit()} · {m.leaderboard_col_originality()}
                {entry.bestOriginality} · {formatTime(entry.avgTimeSeconds)}
              {/if}
            </span>
          </span>
          <span class="kd-cell kd-cell--right kd-cell--desktop tabular-nums text-fg-muted text-sm">
            {entry.wins}
          </span>
          <span class="kd-cell kd-cell--right kd-cell--desktop tabular-nums text-fg-muted text-sm">
            {entry.gamesCount}
          </span>
          <span class="kd-cell kd-cell--right kd-cell--desktop tabular-nums text-fg-muted text-sm">
            {formatTime(entry.avgTimeSeconds)}
          </span>
          <span class="kd-cell kd-cell--right kd-cell--desktop tabular-nums text-fg-muted text-sm">
            {entry.bestOriginality}
          </span>
          <span class="kd-cell kd-cell--right font-display tabular-nums text-fg text-base">
            {entry.bestScore}
          </span>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .kd-leaderboard {
    contain: layout style;
  }

  /* Tabs — pill row, scrolls horizontally on narrow viewports if labels grow
     in another locale. Default fits 360px just fine. */
  .kd-tabs {
    display: flex;
    gap: 0.35rem;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .kd-tabs::-webkit-scrollbar {
    display: none;
  }
  .kd-tab {
    flex: 0 0 auto;
    padding: 0.45rem 0.85rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--color-fg-muted);
    background: var(--color-bg-subtle);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    transition:
      color 160ms var(--ease-out),
      background-color 160ms var(--ease-out),
      border-color 160ms var(--ease-out);
  }
  .kd-tab:hover {
    color: var(--color-fg);
  }
  .kd-tab.active {
    color: var(--color-accent-fg);
    background: var(--color-accent);
    border-color: var(--color-accent);
  }
  .kd-tab:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }

  /* Grid columns: mobile = 3 cols (rank, player+sub, score). Desktop = 7
     cols (rank, player, wins, count, time, orig, score). Hidden columns on
     mobile fully collapse via `display: none` rather than 0 width so the
     auto-fit grid doesn't reserve empty tracks. */
  .kd-head,
  .kd-row {
    display: grid;
    grid-template-columns: 3rem 1fr 4rem;
  }
  @media (min-width: 640px) {
    .kd-head,
    .kd-row {
      grid-template-columns: 3rem 1fr 4rem 4rem 4.5rem 4.5rem 5rem;
    }
  }
  .kd-cell--desktop {
    display: none;
  }
  @media (min-width: 640px) {
    .kd-cell--desktop {
      display: inline-block;
    }
  }
  .kd-cell--right {
    text-align: right;
  }

  /* Sortable headers — same desktop-only visibility rules as the cells. */
  .kd-head__sort {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--color-fg-muted);
    transition: color 140ms var(--ease-out);
  }
  .kd-head__sort:hover {
    color: var(--color-fg);
  }
  .kd-head__sort.active {
    color: var(--color-accent);
  }
  .kd-head__sort--right {
    justify-content: flex-end;
  }
  .kd-head__sort--desktop {
    display: none;
  }
  @media (min-width: 640px) {
    .kd-head__sort--desktop {
      display: inline-flex;
    }
  }

  .kd-row__sub {
    color: var(--color-fg-muted);
    font-size: 11px;
    line-height: 1.2;
    letter-spacing: 0.02em;
  }

  .kd-rank {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: var(--color-fg-muted);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .kd-row--top1 .kd-rank {
    color: oklch(0.78 0.15 80);
  }
  .kd-row--top2 .kd-rank {
    color: oklch(0.72 0.04 80);
  }
  .kd-row--top3 .kd-rank {
    color: oklch(0.62 0.12 50);
  }
  .kd-row {
    border-top: 1px solid transparent;
    border-bottom: 1px solid var(--color-border);
    transition: background-color 160ms var(--ease-out);
  }
  .kd-row:last-child {
    border-bottom: none;
  }
  .kd-row:hover {
    background: color-mix(in oklab, var(--color-fg) 3%, transparent);
  }
  .kd-row--top1 {
    background: color-mix(in oklab, var(--color-signal) 10%, transparent);
  }
  .kd-skel {
    background: color-mix(in oklab, var(--color-fg) 6%, var(--color-bg-subtle));
    border-radius: 4px;
    animation: kd-pulse 1.4s var(--ease-in-out) infinite;
  }
  .kd-skel--rank {
    height: 14px;
    width: 32px;
  }
  .kd-skel--name {
    height: 14px;
    max-width: 200px;
  }
  .kd-skel--score {
    height: 14px;
    width: 40px;
  }
  @keyframes kd-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.55;
    }
  }
</style>
