<script lang="ts">
  /**
   * Inline SVG pastille for a Paris metro or RER line, drawn from the
   * official RATP palette. Use `<LineBadge code="7" />` for métro 7,
   * `<LineBadge code="A" network="rer" />` for RER A.
   *
   * Why inline SVG rather than an icon font or static asset:
   * - Tunable colors via CSS variables (works in dark mode without forking
   *   the palette)
   * - Zero binary asset, zero extra HTTP request
   * - Trivially scalable next to the surrounding type
   * - We're not using the RATP logo wordmark — just the line color disc +
   *   the line code — which is editorial/non-commercial use and stays
   *   within fair use of trademark norms.
   */

  interface Props {
    /** Line identifier: "1".."14" for metro, "A".."E" for RER. */
    code: string;
    /** Network family. Defaults to metro. */
    network?: "metro" | "rer";
    /** Pixel size of the disc. Default 18 — matches a 14 px label baseline. */
    size?: number;
    /** Optional aria-label override. Defaults to "ligne <code>" / "RER <code>". */
    label?: string;
  }

  let { code, network = "metro", size = 18, label }: Props = $props();

  // Source: ratp.fr line color references (publicly documented hex codes).
  // Values are clipped to the trunk lines kalidoku currently supports
  // (M1..14, RER A..E); branches like B2/B4 collapse to their trunk.
  const METRO_COLORS: Record<string, { bg: string; fg: string }> = {
    "1": { bg: "#FFCD00", fg: "#000000" },
    "2": { bg: "#003CA6", fg: "#FFFFFF" },
    "3": { bg: "#837902", fg: "#FFFFFF" },
    "3bis": { bg: "#6ECA97", fg: "#000000" },
    "4": { bg: "#CF009E", fg: "#FFFFFF" },
    "5": { bg: "#FF7E2E", fg: "#000000" },
    "6": { bg: "#6ECA97", fg: "#000000" },
    "7": { bg: "#FA9ABA", fg: "#000000" },
    "7bis": { bg: "#6ECA97", fg: "#000000" },
    "8": { bg: "#E19BDF", fg: "#000000" },
    "9": { bg: "#B6BD00", fg: "#000000" },
    "10": { bg: "#C9910D", fg: "#000000" },
    "11": { bg: "#704B1C", fg: "#FFFFFF" },
    "12": { bg: "#007852", fg: "#FFFFFF" },
    "13": { bg: "#6EC4E8", fg: "#000000" },
    "14": { bg: "#62259D", fg: "#FFFFFF" },
  };

  const RER_COLORS: Record<string, { bg: string; fg: string }> = {
    A: { bg: "#E2231A", fg: "#FFFFFF" },
    B: { bg: "#5291CE", fg: "#FFFFFF" },
    C: { bg: "#FCD946", fg: "#000000" },
    D: { bg: "#00643C", fg: "#FFFFFF" },
    E: { bg: "#A0006E", fg: "#FFFFFF" },
  };

  const colors = $derived.by(() => {
    const table = network === "rer" ? RER_COLORS : METRO_COLORS;
    return table[code] ?? { bg: "#888888", fg: "#FFFFFF" };
  });

  const ariaLabel = $derived(label ?? (network === "rer" ? `RER ${code}` : `ligne ${code}`));

  // RER lines render as a square with rounded corners (consistent with the
  // RATP signage); metro lines render as a disc.
  const shape = $derived(network === "rer" ? "square" : "circle");

  // Smaller font when the code is more than 1 char (e.g. "14", "3bis") so
  // it still fits inside the disc.
  const fontSize = $derived(code.length === 1 ? size * 0.62 : size * 0.5);
</script>

<span
  class="kd-line-badge"
  role="img"
  aria-label={ariaLabel}
  style="--kd-badge-bg: {colors.bg}; --kd-badge-fg: {colors.fg}; width: {size}px; height: {size}px;"
>
  <svg viewBox="0 0 {size} {size}" width={size} height={size} aria-hidden="true" focusable="false">
    {#if shape === "circle"}
      <circle cx={size / 2} cy={size / 2} r={size / 2 - 0.5} fill="var(--kd-badge-bg)" />
    {:else}
      <rect x="0.5" y="0.5" width={size - 1} height={size - 1} rx="2" fill="var(--kd-badge-bg)" />
    {/if}
    <text
      x={size / 2}
      y={size / 2}
      text-anchor="middle"
      dominant-baseline="central"
      font-size={fontSize}
      font-weight="700"
      font-family="var(--font-display, system-ui)"
      fill="var(--kd-badge-fg)"
    >
      {code}
    </text>
  </svg>
</span>

<style>
  .kd-line-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    vertical-align: middle;
    flex: 0 0 auto;
    line-height: 1;
  }
</style>
