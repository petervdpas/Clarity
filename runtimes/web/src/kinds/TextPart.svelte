<script lang="ts">
  import type { PartContext } from "./context.ts";
  import { displayValue } from "../screen/model.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
  const bound = $derived(ctx.bound);
</script>

<span class="text-value" class:lines={p.height > 1} id={ctx.id}>{bound ? displayValue(ctx.preview, bound.field, ctx.value) : ""}</span>

<style>
  .text-value {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Over several rows: lines to read, wrapped from the top, with their
     paragraphs, scrolling when longer than the rows. */
  .text-value.lines {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    overflow: auto;
    text-overflow: clip;
    align-self: stretch;
    height: 100%;
  }
</style>
