<script lang="ts">
  import type { components } from "../api/types.js";

  type Predicate = components["schemas"]["PredicateLabel"];

  interface Props {
    predicate: Predicate;
  }

  let { predicate }: Props = $props();
</script>

<div
  class="bg-bg-subtle border-border flex min-h-12 items-center rounded-md border px-1.5 py-1 text-[10.5px] leading-snug sm:px-2 sm:text-xs"
  title={predicate.help ?? predicate.label}
>
  <span
    class="text-fg-subtle block w-full text-center font-medium break-words hyphens-auto sm:text-left"
    lang="fr"
  >
    {predicate.label}
  </span>
</div>
