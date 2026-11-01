<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { displayValue } from "../screen/model.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const bound = $derived(ctx.bound);
</script>

<span class="badge {ctx.partClass ?? ''}" id={ctx.id}>{bound ? displayValue(ctx.preview, bound.field, ctx.value) : (ctx.caption ?? "")}</span>

<style>
  .badge {
    align-self: center;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: var(--font-size-small);
    white-space: nowrap;
  }
</style>
