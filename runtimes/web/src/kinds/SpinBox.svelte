<script lang="ts">
  import type { PartContext } from "./context.ts";
  import NumberInput from "../parts/NumberInput.svelte";
  import { fieldFormat, formatsOf, tr } from "../screen/model.ts";
  import { numberEditing } from "../screen/format.ts";
  import { numberRange, type RangeMove } from "../screen/ranges.ts";
  import { wiredCall } from "../screen/calls.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const live = $derived(ctx.live);
  const bound = $derived(ctx.bound);

  // A spin box shows its value in the person's format and reads what is
  // typed the same way; unbound, or without a format, it keeps the text.
  const numberFormat = $derived(bound ? fieldFormat(bound.field) : null);
  const numbers = $derived(
    numberFormat ? numberEditing(numberFormat, formatsOf(ctx.preview)) : { text: (v: unknown) => String(v ?? ""), read: (typed: string) => typed },
  );

  // A spin box's range: as written, else the bound field's own.
  const range = $derived(numberRange(p, bound?.field.modifiers));
  const whole = $derived(bound ? bound.field.type.kind === "integer" : !range.step.includes(".") && !(range.min ?? "").includes("."));

  /**
   * What the person did to a bound spin box, in a running application: the
   * runtime makes the value within the range and runs `on change`, as one
   * edit (DESIGN 1.7). Elsewhere the input makes it.
   */
  const rangeMoved = $derived(
    live?.range && p.name && p.bind && bound
      ? (move: RangeMove) => void live.range!(p.name!, { path: p.bind!, min: range.min, max: range.max, step: range.step, whole, ...move }, wiredCall(p.handlers, "change"))
      : undefined,
  );
</script>

<NumberInput
  id={ctx.id}
  value={bound ? ctx.value : ctx.shown}
  text={numbers.text}
  read={numbers.read}
  placeholder={p.placeholder ? tr(ctx.preview, p.placeholder) : undefined}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  {range}
  {whole}
  onrange={rangeMoved}
  onchange={(next) => {
    if (bound) ctx.setValue(next);
    else ctx.setText(String(next));
    ctx.fire("change");
  }}
  onsubmit={ctx.submit}
  onfocus={() => ctx.fire("enter")}
  onblur={() => ctx.fire("leave")}
/>
