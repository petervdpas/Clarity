<script lang="ts">
  import type { PartContext } from "./context.ts";
  import SliderInput from "../parts/SliderInput.svelte";
  import { numberRange, type RangeMove } from "../screen/ranges.ts";
  import { wiredCall } from "../screen/calls.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const live = $derived(ctx.live);
  const bound = $derived(ctx.bound);

  // A slider's range: as written, else the bound field's own.
  const range = $derived(numberRange(p, bound?.field.modifiers));
  const whole = $derived(bound ? bound.field.type.kind === "integer" : !range.step.includes(".") && !(range.min ?? "").includes("."));

  /**
   * What the person did to a bound slider, in a running application: the
   * runtime makes the value within the range and runs `on change`, as one
   * edit (DESIGN 1.7). Elsewhere the input makes it.
   */
  const rangeMoved = $derived(
    live?.range && p.name && p.bind && bound
      ? (move: RangeMove) => void live.range!(p.name!, { path: p.bind!, min: range.min, max: range.max, step: range.step, whole, ...move }, wiredCall(p.handlers, "change"))
      : undefined,
  );
</script>

<SliderInput
  id={ctx.id}
  value={bound ? (ctx.value as string | number | null) : ctx.shown}
  {range}
  {whole}
  onrange={rangeMoved}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    if (bound) ctx.setValue(next);
    else ctx.setText(String(next));
    ctx.fire("change");
  }}
  onfocus={() => ctx.fire("enter")}
  onblur={() => ctx.fire("leave")}
/>
