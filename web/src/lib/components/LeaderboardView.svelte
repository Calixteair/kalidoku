<script lang="ts">
  import Crown from "lucide-svelte/icons/crown";
  import Medal from "lucide-svelte/icons/medal";
  import Trophy from "lucide-svelte/icons/trophy";
  import { api, ApiError } from "../api/client.js";
  import type { components } from "../api/types.js";
  import * as m from "../../paraglide/messages.js";

  type LeaderboardPage = components["schemas"]["LeaderboardPage"];

  interface Props {
    domain: string;
  }

  let { domain }: Props = $props();

  let page = $state<LeaderboardPage | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const load = async (): Promise<void> => {
    loading = true;
    error = null;
    try {
      page = await api.get("/api/leaderboard/{domain}/today", { domain });
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        page = { items: [] };
      } else {
        error = m.error_network();
      }
    } finally {
      loading = false;
    }
  };

  $effect(() => {
    void load();
  });

  type RankIcon = typeof Crown;
  const rankIcon = (rank: number): RankIcon | null => {
    if (rank === 1) return Crown;
    if (rank === 2) return Trophy;
    if (rank === 3) return Medal;
    return null;
  };
</script>

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
    <li
      class="bg-bg-subtle grid grid-cols-[3rem_1fr_4rem] items-center gap-3 px-4 py-2.5 border-b border-border"
    >
      <span class="eyebrow text-fg-muted">{m.page_leaderboard_rank_col()}</span>
      <span class="eyebrow text-fg-muted">{m.page_leaderboard_player_col()}</span>
      <span class="eyebrow text-fg-muted text-right">{m.page_leaderboard_score_col()}</span>
    </li>
    {#each page.items as entry (entry.rank)}
      {@const Icon = rankIcon(entry.rank)}
      <li
        class="kd-row grid grid-cols-[3rem_1fr_4rem] items-center gap-3 px-4 py-3"
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
        <span class="text-fg truncate text-sm font-medium">{entry.profile.pseudo}</span>
        <span class="text-fg font-display tabular-nums text-right text-base">{entry.score}</span>
      </li>
    {/each}
  </ol>
{/if}

<style>
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
