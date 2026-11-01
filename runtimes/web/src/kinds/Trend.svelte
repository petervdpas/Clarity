<script lang="ts">
  import TrendView from "../parts/TrendView.svelte";
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<!-- A trend (DESIGN 5.26): the bound number's last values, taken while the window runs. -->
<TrendView value={ctx.value} points={p.points} every={p.every} top={p.max ? Number(p.max) : null} running={!!ctx.live && !ctx.design} label={p.name ?? undefined} />
