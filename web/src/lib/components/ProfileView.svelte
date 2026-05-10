<script lang="ts">
  import { onMount } from "svelte";
  import * as m from "../../paraglide/messages.js";
  import { createAuthStore } from "../stores/authStore.svelte.js";

  const auth = createAuthStore();

  onMount(() => {
    void auth.refresh();
  });

  const onLogout = async (): Promise<void> => {
    await auth.logout();
    window.location.assign("/");
  };

  const loginUrl = "/api/auth/login?redirect_to=/profile";
</script>

{#if auth.loading || !auth.loaded}
  <p class="text-fg-muted text-sm">{m.loading_stations()}</p>
{:else if auth.me === null}
  <div class="flex flex-col gap-2">
    <p class="text-fg-muted text-sm">{m.page_profile_anonymous()}</p>
    <a
      href={loginUrl}
      class="bg-accent text-accent-fg inline-flex w-fit items-center rounded-md px-4 py-2 text-sm font-semibold"
    >
      {m.page_profile_login()}
    </a>
  </div>
{:else}
  <dl class="border-border bg-bg-card divide-border grid divide-y rounded-lg border text-sm">
    <div class="flex items-center justify-between gap-2 px-3 py-2">
      <dt class="text-fg-muted">{m.page_profile_pseudo()}</dt>
      <dd class="text-fg font-medium">{auth.me.pseudo}</dd>
    </div>
    <div class="flex items-center justify-between gap-2 px-3 py-2">
      <dt class="text-fg-muted">{m.page_profile_email()}</dt>
      <dd class="text-fg font-medium">{auth.me.email}</dd>
    </div>
  </dl>
  <button
    type="button"
    class="border-border bg-bg-card text-fg mt-3 inline-flex items-center rounded-md border px-4 py-2 text-sm font-semibold"
    onclick={() => void onLogout()}
  >
    {m.page_profile_logout()}
  </button>
{/if}

<style>
  button,
  a {
    min-height: 44px;
  }
</style>
