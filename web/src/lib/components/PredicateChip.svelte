<script lang="ts">
  import type { components } from "../api/types.js";
  import LineBadge from "./LineBadge.svelte";

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
    /** Active domain id — drives which line-badge palette to use. */
    domain: string;
  }

  let { predicate, orientation, index, marker, domain }: Props = $props();

  // Tokenise the predicate label so an "on_attr_in_set" line predicate
  // ("Sur la ligne 7") inlines a coloured pastille instead of the plain
  // digit. Other predicates pass through unchanged.
  //
  // The match works on the predicate family + a regex over the localised
  // text so it survives FR/EN translation. We don't try to be clever about
  // partial sentences — the labels in our packs follow a consistent
  // "ligne X" / "line X" / "RER X" shape.
  type Segment = { kind: "text"; value: string } | { kind: "badge"; code: string };

  const network: "metro" | "rer" = $derived(domain === "rer" ? "rer" : "metro");

  // Matches "ligne 7", "line 14", "RER A", case-insensitive, capturing the
  // line code (digit+optional 'bis' for metro, single letter for RER).
  const LINE_PATTERN = /\b(?:lignes?|lines?|RER)\s+([A-E]|\d{1,2}(?:\s?bis)?)\b/giu;

  const tokenise = (label: string, family: string): Segment[] => {
    if (family !== "on_attr_in_set") {
      return [{ kind: "text", value: label }];
    }
    const out: Segment[] = [];
    let cursor = 0;
    LINE_PATTERN.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = LINE_PATTERN.exec(label)) !== null) {
      const start = match.index;
      const end = start + match[0].length;
      // The prefix word ("ligne" / "RER") + space stays in the text; the
      // code itself becomes a badge. Find the boundary inside the match.
      const codeStart = match[0].search(/[A-E\d]/i);
      if (codeStart < 0) continue;
      const prefix = label.slice(cursor, start + codeStart);
      const code = match[1].replace(/\s+/g, "").toLowerCase();
      out.push({ kind: "text", value: prefix });
      out.push({ kind: "badge", code });
      cursor = end;
    }
    if (cursor === 0) {
      return [{ kind: "text", value: label }];
    }
    if (cursor < label.length) {
      out.push({ kind: "text", value: label.slice(cursor) });
    }
    return out;
  };

  const segments = $derived(tokenise(predicate.label, predicate.family));
</script>

<div
  class="kd-chip ticket"
  class:row={orientation === "row"}
  class:col={orientation === "col"}
  title={predicate.help ?? predicate.label}
>
  <span class="tag eyebrow" aria-hidden="true">{marker}{index}</span>
  <span class="text-fg-subtle label" lang="fr">
    {#each segments as seg, i (i)}
      {#if seg.kind === "text"}{seg.value}{:else}<LineBadge
          code={seg.code}
          {network}
          size={15}
        />{/if}
    {/each}
  </span>
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
