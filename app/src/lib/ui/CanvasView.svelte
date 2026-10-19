<script lang="ts">
  // A canvas drawn by code: the shapes its draw handler sent, in SVG, in the
  // canvas's own units, scaled to fit the part and centered. Avalonia's
  // CanvasView draws the same (conformance/drawing.json).
  import { svgOf, type Shape } from "../drawing";
  import type { ShownText } from "../format";

  let {
    drawing,
    units,
    shown,
    id,
  }: { drawing: Shape[]; units: { width: number; height: number }; shown: (text: ShownText) => string; id?: string } = $props();

  const elements = $derived(drawing.map((shape) => svgOf(shape, units, shown)));
</script>

<svg {id} class="canvas-view" viewBox="0 0 {units.width} {units.height}" preserveAspectRatio="xMidYMid meet" role="img">
  {#each elements as element, i (i)}
    <svelte:element this={element.tag} {...element.attrs}>{element.text ?? ""}</svelte:element>
  {/each}
</svg>

<style>
  .canvas-view {
    width: 100%;
    height: 100%;
    display: block;
    font-family: var(--font);
  }
</style>
