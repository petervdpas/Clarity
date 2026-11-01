<script lang="ts">
  import type { PartContext } from "./context.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<span class="separator-line" class:vertical={p.orientation === "vertical"} role="separator" aria-orientation={p.orientation === "vertical" ? "vertical" : "horizontal"}></span>

<style>
  .separator-line {
    flex: 1;
    align-self: center;
    height: var(--border-width, 1px);
    background: var(--border-strong);
  }

  .separator-line.vertical {
    flex: none;
    align-self: stretch;
    width: var(--border-width, 1px);
    height: auto;
    margin: 0 auto;
  }
</style>
