<script lang="ts">
  import type { PartContext } from "./context.ts";
  import RadioInput from "../parts/RadioInput.svelte";
  import { choicesOf } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<RadioInput
  id={ctx.id}
  name="radio-{ctx.offset}"
  value={ctx.value}
  choices={ctx.bound ? (choicesOf(ctx, ctx.bound.field) ?? []) : []}
  vertical={p.orientation === "vertical"}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled || ctx.readonly}
  onchange={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
/>
