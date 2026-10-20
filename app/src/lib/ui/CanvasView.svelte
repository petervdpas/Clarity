<script lang="ts">
  // A canvas drawn by code: the shapes its draw handler sent, in SVG, in the
  // canvas's own units, scaled to fit the part and centered. Avalonia's
  // CanvasView draws the same (conformance/drawing.json). With `onpoint`, a
  // click (mouse, pen or touch) gives where in the canvas's units, found
  // when it happens, so it holds however the part is sized.
  import { pointOf, svgOf, type Shape } from "../drawing";
  import type { ShownText } from "../format";

  let {
    drawing,
    units,
    shown,
    id,
    onpoint = null,
  }: {
    drawing: Shape[];
    units: { width: number; height: number };
    shown: (text: ShownText) => string;
    id?: string;
    /** A click on the drawing, in the canvas's units; none for a click beside it. */
    onpoint?: ((point: { x: number; y: number }) => void) | null;
  } = $props();

  const elements = $derived(drawing.map((shape) => svgOf(shape, units, shown)));

  /** The pointer that went down on the canvas: a click is it coming up again. */
  let pressed: number | null = null;

  function down(e: PointerEvent) {
    if (onpoint && e.isPrimary && e.button === 0) pressed = e.pointerId;
  }

  function up(e: PointerEvent) {
    if (!onpoint || pressed !== e.pointerId) return;
    pressed = null;
    const box = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
    const point = pointOf(units, { width: box.width, height: box.height }, { x: e.clientX - box.left, y: e.clientY - box.top });
    if (point) onpoint(point);
  }
</script>

<svg
  {id}
  class="canvas-view"
  class:clickable={onpoint !== null}
  viewBox="0 0 {units.width} {units.height}"
  preserveAspectRatio="xMidYMid meet"
  role="img"
  onpointerdown={down}
  onpointerup={up}
  onpointercancel={() => (pressed = null)}
>
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

  .canvas-view.clickable {
    cursor: pointer;
    touch-action: manipulation;
  }
</style>
