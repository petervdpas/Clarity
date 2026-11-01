<script lang="ts">
  import type { PartContext } from "./context.ts";
  import DateInput from "../parts/DateInput.svelte";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<DateInput
  id={ctx.id}
  value={ctx.value}
  tabindex={p.tabOrder ?? undefined}
  describedBy={ctx.describedBy}
  invalid={ctx.error !== null}
  disabled={ctx.disabled}
  readonly={ctx.readonly}
  onchange={(next) => {
    ctx.setValue(next);
    ctx.fire("change");
  }}
  onsubmit={ctx.submit}
/>
