<script lang="ts">
  import type { PartContext } from "./context.ts";
  import ColorInput from "../parts/ColorInput.svelte";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<ColorInput
  id={ctx.id}
  value={ctx.bound ? (ctx.value as string | null) : ctx.shown}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    if (ctx.bound) ctx.setValue(next);
    else ctx.setText(next);
    ctx.fire("change");
  }}
/>
