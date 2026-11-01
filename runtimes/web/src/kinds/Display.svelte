<script lang="ts">
  import type { PartContext } from "./context.ts";
  import SegmentDisplay from "../parts/SegmentDisplay.svelte";
  import { DEFAULT_DIGITS } from "../screen/display.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<SegmentDisplay id={ctx.id} value={ctx.bound ? ctx.value : null} digits={p.digits ?? DEFAULT_DIGITS} />
