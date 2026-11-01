<script lang="ts">
  import type { PartContext } from "./context.ts";
  import NumberInput from "../parts/NumberInput.svelte";
  import { fieldFormat, formatsOf, tr } from "../screen/model.ts";
  import { numberEditing } from "../screen/format.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);

  // A number input shows its value in the person's format and reads what is
  // typed the same way; unbound, or without a format, it keeps the text.
  const numberFormat = $derived(ctx.bound ? fieldFormat(ctx.bound.field) : null);
  const numbers = $derived(
    numberFormat ? numberEditing(numberFormat, formatsOf(ctx.preview)) : { text: (v: unknown) => String(v ?? ""), read: (typed: string) => typed },
  );
</script>

<NumberInput
  id={ctx.id}
  value={ctx.bound ? ctx.value : ctx.shown}
  text={numbers.text}
  read={numbers.read}
  placeholder={p.placeholder ? tr(ctx.preview, p.placeholder) : undefined}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    if (ctx.bound) ctx.setValue(next);
    else ctx.setText(String(next));
    ctx.fire("change");
  }}
  onsubmit={ctx.submit}
  onfocus={() => ctx.fire("enter")}
  onblur={() => ctx.fire("leave")}
/>
