<script lang="ts">
  import type { components } from "../api/types.js";

  type Predicate = components["schemas"]["PredicateLabel"];
  type Orientation = "row" | "col";

  interface Props {
    predicate: Predicate;
    /** "row" → horizontal label rendered left of the grid; "col" → above the grid. */
    orientation: Orientation;
    /** 1-based index for the visual tag (L1/L2/L3 or C1/C2/C3). */
    index: number;
    /** Marker letter ("L" rows, "C" columns) — already localised by the caller. */
    marker: string;
  }

  let { predicate, orientation, index, marker }: Props = $props();
</script>

<div
  class="kd-chip ticket"
  class:row={orientation === "row"}
  class:col={orientation === "col"}
  title={predicate.help ?? predicate.label}
>
  <span class="tag eyebrow" aria-hidden="true">{marker}{index}</span>
  <span class="text-fg-subtle label" lang="fr">{predicate.label}</span>
</div>

<style>
  .kd-chip {
    --bar: color-mix(in oklab, var(--color-accent) 80%, transparent);
    position: relative;
    display: flex;
    border-radius: var(--radius-md);
    overflow: hidden;
    min-height: 64px;
    padding: 0.45rem 0.55rem;
  }
  /* Transit-line accent bar — 3px stripe glued to the inner edge of the chip,
     gives the predicate a "platform sign" feel. */
  .kd-chip::before {
    content: "";
    position: absolute;
    background: var(--bar);
    border-radius: 999px;
  }
  .kd-chip.row {
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    gap: 0.2rem;
    padding-left: 0.75rem;
  }
  .kd-chip.row::before {
    left: 4px;
    top: 8px;
    bottom: 8px;
    width: 3px;
  }
  .kd-chip.col {
    flex-direction: column;
    align-items: stretch;
    justify-content: flex-end;
    gap: 0.2rem;
    padding-bottom: 0.55rem;
  }
  .kd-chip.col::before {
    bottom: 4px;
    left: 8px;
    right: 8px;
    height: 3px;
  }
  .tag {
    color: var(--color-fg-muted);
    font-size: 9.5px;
    letter-spacing: 0.18em;
  }
  .label {
    font-size: 11px;
    line-height: 1.25;
    font-weight: 500;
    overflow-wrap: anywhere;
    text-wrap: balance;
    hyphens: auto;
    -webkit-hyphens: auto;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  /* Columns get a tighter font and centred text — top of a vertical sign */
  .kd-chip.col .label {
    text-align: center;
    font-size: 10.5px;
  }
  .kd-chip.row .label {
    text-align: left;
  }
  /* Above 640px the row chips have more room → a touch larger */
  @media (min-width: 640px) {
    .kd-chip.row .label,
    .kd-chip.col .label {
      font-size: 12px;
    }
    .kd-chip {
      min-height: 72px;
    }
  }
</style>
