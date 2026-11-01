<script lang="ts">
  import type { PartContext } from "./context.ts";

  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<div class="scroll-box">
  {@render ctx.grid({ widgets: ctx.widget.children, columns: p.width, container: ctx.offset })}
</div>

<style>
  /* The scroll bar stands in the space between columns, so the parts keep the window's column lines. */
  .scroll-box {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    margin-right: -8px;
    padding-right: 8px;
    scrollbar-width: thin;
  }
</style>
