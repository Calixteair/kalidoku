<script lang="ts">
  import * as m from "../../paraglide/messages.js";
  import type { components } from "../api/types.js";

  type Predicate = components["schemas"]["PredicateLabel"];

  interface Props {
    predicate: Predicate;
    onHelp?: ((p: Predicate) => void) | undefined;
  }

  let { predicate, onHelp }: Props = $props();

  const handleHelp = (): void => {
    if (onHelp) onHelp(predicate);
  };
</script>

<div
  class="bg-bg-subtle border-border flex items-center justify-between gap-1 rounded-md border px-2 py-1 text-xs leading-tight"
>
  <span class="line-clamp-2 text-fg-subtle font-medium">{predicate.label}</span>
  {#if predicate.help}
    <button
      type="button"
      class="text-fg-muted hover:text-fg flex h-6 w-6 shrink-0 items-center justify-center rounded-full"
      aria-label={m.modal_predicate_help()}
      onclick={handleHelp}
    >
      ?
    </button>
  {/if}
</div>
