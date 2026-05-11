<script lang="ts">
  import LogIn from "lucide-svelte/icons/log-in";
  import LogOut from "lucide-svelte/icons/log-out";
  import Mail from "lucide-svelte/icons/mail";
  import UserRound from "lucide-svelte/icons/user-round";
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

  const initial = $derived((auth.me?.pseudo ?? "?").trim().slice(0, 1).toUpperCase());
</script>

{#if auth.loading || !auth.loaded}
  <div class="kd-skel kd-skel--card" aria-hidden="true"></div>
  <p class="sr-only">{m.loading_stations()}</p>
{:else if auth.me === null}
  <div class="surface flex flex-col items-start gap-4 rounded-xl p-5">
    <div class="flex h-14 w-14 items-center justify-center rounded-full bg-bg-subtle">
      <UserRound size={28} aria-hidden="true" class="text-fg-muted" />
    </div>
    <div class="flex flex-col gap-1">
      <p class="font-display text-fg text-xl font-semibold">{m.page_profile_anonymous()}</p>
      <p class="text-fg-subtle text-sm leading-relaxed">{m.page_profile_anonymous_hint()}</p>
    </div>
    <a href={loginUrl} class="btn btn-primary">
      <LogIn size={16} aria-hidden="true" />
      <span>{m.page_profile_login()}</span>
    </a>
  </div>
{:else}
  <div class="surface flex items-center gap-4 rounded-xl p-5">
    <div class="kd-avatar font-display" aria-hidden="true">{initial}</div>
    <div class="flex flex-col">
      <p class="eyebrow text-fg-muted">{m.page_profile_pseudo()}</p>
      <p class="font-display text-fg text-xl font-semibold leading-tight">{auth.me.pseudo}</p>
      <p class="text-fg-muted mt-0.5 inline-flex items-center gap-1 text-xs">
        <Mail size={11} aria-hidden="true" />
        <span>{auth.me.email}</span>
      </p>
    </div>
  </div>

  <button type="button" class="btn btn-secondary mt-4" onclick={() => void onLogout()}>
    <LogOut size={16} aria-hidden="true" />
    <span>{m.page_profile_logout()}</span>
  </button>
{/if}

<style>
  .kd-skel {
    background: color-mix(in oklab, var(--color-fg) 6%, var(--color-bg-subtle));
    border-radius: var(--radius-xl);
    animation: kd-pulse 1.4s var(--ease-in-out) infinite;
  }
  .kd-skel--card {
    height: 124px;
  }
  .kd-avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 3.5rem;
    height: 3.5rem;
    border-radius: 999px;
    background: var(--color-accent);
    color: var(--color-accent-fg);
    font-size: 1.5rem;
    font-weight: 700;
    font-variation-settings:
      "opsz" 32,
      "SOFT" 40;
    box-shadow: var(--shadow-paper);
  }
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.7rem 1rem;
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    font-weight: 600;
    min-height: 44px;
    border: 1px solid transparent;
    transition:
      background-color 160ms var(--ease-out),
      transform 160ms var(--ease-out),
      box-shadow 160ms var(--ease-out);
  }
  .btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--color-ring);
  }
  .btn-primary {
    background: var(--color-accent);
    color: var(--color-accent-fg);
    box-shadow: var(--shadow-paper);
  }
  .btn-primary:hover {
    transform: translateY(-1px);
    box-shadow: var(--shadow-lift);
  }
  .btn-secondary {
    background: var(--color-bg-card);
    color: var(--color-fg);
    border-color: var(--color-border);
  }
  .btn-secondary:hover {
    border-color: var(--color-border-strong);
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
