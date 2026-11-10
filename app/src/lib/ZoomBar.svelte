<script lang="ts">
  // The zoom bar every diagram canvas shows at its top left, where the
  // properties drawer never lies over it: zoom out, the level (back to
  // 100%), zoom in, center, and what the canvas adds (Arrange).
  import type { Snippet } from "svelte";

  let {
    zoom,
    onzoom,
    onreset,
    oncenter,
    children,
  }: {
    zoom: number;
    /** Zooms by `factor` (above 1 in, below 1 out). */
    onzoom: (factor: number) => void;
    /** Back to 100%. */
    onreset: () => void;
    /** Puts the drawing in the middle of the room left free. */
    oncenter: () => void;
    /** More buttons, after the zoom ones. */
    children?: Snippet;
  } = $props();

  const STEP = 1.2;
</script>

<div class="zoom">
  <button onclick={() => onzoom(1 / STEP)} aria-label="Zoom out" title="Zoom out (Ctrl+wheel)">-</button>
  <button class="level" onclick={onreset} title="Back to 100%">{Math.round(zoom * 100)}%</button>
  <button onclick={() => onzoom(STEP)} aria-label="Zoom in" title="Zoom in (Ctrl+wheel)">+</button>
  <button class="center" onclick={oncenter} aria-label="Center the diagram" title="Center the diagram in view">
    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8" /><circle cx="12" cy="12" r="3" /><path d="M12 1v4M12 19v4M1 12h4M19 12h4" /></svg>
  </button>
  {@render children?.()}
</div>

<style>
  .zoom {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 1;
    display: flex;
    gap: 2px;
  }

  .zoom :global(button) {
    min-width: 28px;
    padding: 2px 6px;
    font-size: 12px;
  }

  .level {
    min-width: 52px;
    font-variant-numeric: tabular-nums;
  }

  .center {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
</style>
