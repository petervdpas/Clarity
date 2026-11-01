<script lang="ts">
  import type { PartContext } from "./context.ts";
  let { ctx }: { ctx: PartContext } = $props();
  const p = $derived(ctx.p);
</script>

<div class="placeholder canvas">
  <span>Canvas ({p.surface ?? "svg"})</span>
</div>

<style>
  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: var(--border-width) dashed var(--border-strong);
    border-radius: var(--radius-panel);
    color: var(--text-muted);
    font-size: var(--font-size-small);
  }

  .placeholder.canvas {
    background-image:
      linear-gradient(45deg, var(--panel-alt) 25%, transparent 25%),
      linear-gradient(-45deg, var(--panel-alt) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, var(--panel-alt) 75%),
      linear-gradient(-45deg, transparent 75%, var(--panel-alt) 75%);
    background-size: 16px 16px;
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  }
</style>
