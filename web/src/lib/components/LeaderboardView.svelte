<script lang="ts">
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
</script>

{#if loading}
  <p class="text-fg-muted text-sm">{m.loading_stations()}</p>
{:else if error}
  <p class="text-danger text-sm" role="alert">{error}</p>
{:else if !page || page.items.length === 0}
  <p class="text-fg-muted text-sm">{m.page_leaderboard_empty()}</p>
{:else}
  <ol class="border-border bg-bg-card divide-border divide-y rounded-lg border">
    {#each page.items as entry (entry.rank)}
      <li class="flex items-center justify-between gap-3 px-3 py-2 text-sm">
        <span class="text-fg-muted w-8 text-right tabular-nums">#{entry.rank}</span>
        <span class="text-fg flex-1 truncate font-medium">{entry.profile.pseudo}</span>
        <span class="text-fg tabular-nums">{entry.score}</span>
      </li>
    {/each}
  </ol>
{/if}
