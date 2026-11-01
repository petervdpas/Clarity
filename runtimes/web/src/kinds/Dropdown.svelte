<script lang="ts">
  import type { PartContext } from "./context.ts";
  import ChoiceInput from "../parts/ChoiceInput.svelte";
  import { choicesOf } from "./shared.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<ChoiceInput
  id={ctx.id}
  value={ctx.value}
  choices={ctx.bound ? (choicesOf(ctx, ctx.bound.field) ?? []) : [{ value: null, label: "" }]}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled || ctx.readonly}
  onchange={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
/>
