<script lang="ts">
  import ChartView from "../parts/ChartView.svelte";
  import { numbersOf } from "../parts/trend.ts";
  import type { PartContext } from "./context.ts";
  import { declareView, viewFilled } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const filled = $derived(viewFilled(ctx));

  // A chart of a list of numbers: its last `points` of them, for the runtime to fill in (DESIGN 1.7).
  $effect(() => {
    if (!ctx.live || ctx.design || !p.name || !p.bind) return;
    declareView(ctx, { kind: "numbers", list: p.bind, points: p.points ?? null });
  });
</script>

<!-- A chart of a list of numbers (DESIGN 5.26): drawn as they are, the last points of them. -->
<ChartView
  kind={p.chart === "bar" ? "bar" : "line"}
  label={p.name ?? undefined}
  top={p.max ? Number(p.max) : null}
  groups={(Array.isArray(filled?.numbers) ? (filled.numbers as number[]) : numbersOf(ctx.value, p.points)).map((v) => ({ label: "", value: v, text: String(v) }))}
/>
