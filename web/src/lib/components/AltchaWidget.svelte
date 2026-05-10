<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    challengeUrl?: string;
    name?: string;
    /** Auto-solve on widget load. Default 'onload' for invisible PoW. */
    auto?: "onload" | "onsubmit" | "off";
    onVerified?: ((solution: string) => void) | undefined;
  }

  let {
    challengeUrl = "/api/altcha/challenge",
    name = "altchaSolution",
    auto = "onload",
    onVerified,
  }: Props = $props();

  let host = $state<HTMLElement | null>(null);

  onMount(() => {
    let cleanup: (() => void) | undefined;
    void (async () => {
      try {
        // Dynamic import keeps the widget out of the critical bundle.
        await import("altcha");
      } catch (err) {
        // Stay silent in dev where the package may not have published its
        // global element yet — the widget remains a no-op.
        console.warn("altcha import failed", err);
        return;
      }
      const el = host;
      if (!el) return;
      const handler = (ev: Event): void => {
        const detail = (ev as CustomEvent<{ payload?: string } | string>).detail;
        const payload =
          typeof detail === "string"
            ? detail
            : detail && typeof detail === "object" && "payload" in detail
              ? detail.payload
              : undefined;
        if (typeof payload === "string" && onVerified) {
          onVerified(payload);
        }
      };
      el.addEventListener("verified", handler);
      cleanup = (): void => {
        el.removeEventListener("verified", handler);
      };
    })();
    return () => {
      if (cleanup) cleanup();
    };
  });
</script>

<!-- Invisible Altcha widget. Self-hosted, no third-party calls. -->
<div bind:this={host} class="altcha-host" aria-hidden="true">
  {#if typeof window !== "undefined"}
    <!-- @ts-expect-error custom element provided by altcha package -->
    <altcha-widget challengeurl={challengeUrl} hidefooter hidelogo {auto} {name}></altcha-widget>
  {/if}
</div>

<style>
  .altcha-host {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
