<script lang="ts">
  /**
   * Renders a per-entity image from /cards/<domain>/<entityId>.png, with
   * graceful fallbacks when the asset isn't on disk. Built for Clash Royale
   * card art, but works for any domain that ships icons at the canonical
   * path (future world-airports flags, etc.).
   *
   * Loading strategy:
   * - lazy by default (loading="lazy") so the autocomplete suggestion list
   *   doesn't fire 8 requests on each keystroke before the user actually
   *   scrolls.
   * - on error (404, network, etc.), the <img> hides itself — callers can
   *   render a sibling fallback (e.g. the entity name in text) without us
   *   blocking the layout.
   */

  interface Props {
    domain: string;
    entityId: string;
    /** Display label, used as alt text and the visible fallback. */
    alt: string;
    /** Pixel size of the square box. Default 32 = autocomplete row size. */
    size?: number;
    /** Force eager loading (e.g. on the EndGameModal where they're already
     *  visible). */
    eager?: boolean;
  }

  let { domain, entityId, alt, size = 32, eager = false }: Props = $props();

  let failed = $state(false);

  const src = $derived(`/cards/${domain}/${entityId}.png`);

  const onError = (): void => {
    failed = true;
  };
</script>

{#if !failed}
  <img
    {src}
    {alt}
    width={size}
    height={size}
    loading={eager ? "eager" : "lazy"}
    decoding="async"
    class="kd-card-icon"
    style="width: {size}px; height: {size}px;"
    onerror={onError}
  />
{/if}

<style>
  .kd-card-icon {
    flex: 0 0 auto;
    object-fit: contain;
    border-radius: var(--radius-sm, 4px);
    /* The Supercell card art is 285×420; aspect ratio is portrait. We
       constrain to a square box and let object-fit handle the letterboxing.
       A soft background keeps transparent regions readable in both themes. */
    background: color-mix(in oklab, var(--color-fg) 4%, transparent);
  }
</style>
