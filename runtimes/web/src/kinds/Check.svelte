<script lang="ts">
  import type { PartContext } from "./context.ts";
  import CheckInput from "../parts/CheckInput.svelte";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<CheckInput
  id={ctx.id}
  switch={p.kind === "switch"}
  caption={ctx.caption ?? undefined}
  checked={ctx.bound ? ctx.value === true : ctx.partState?.checked === true}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled || ctx.readonly}
  onchange={(next) => {
    if (ctx.bound) ctx.setValue(next);
    else if (ctx.live && p.name && !ctx.design) ctx.live.setChecked(p.name, next);
    ctx.fire("change");
  }}
/>
