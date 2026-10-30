<script lang="ts">
  // A canvas drawn by code: the shapes its draw handler sent, in SVG, in the
  // canvas's own units, scaled to fit the part and centered. Avalonia's
  // CanvasView draws the same (conformance/drawing.json). With `onpoint`, a
  // click (mouse, pen or touch) gives where in the canvas's units, found
  // when it happens, so it holds however the part is sized.
  //
  // With `zoom` the part shows a view of the units (conformance/drawing.json,
  // views): the wheel scrolls it (with Shift sideways), Ctrl or Cmd with the
  // wheel zooms around the pointer, as a touchpad's pinch does, and the bars
  // say where the view is. With `ondrag` a press that moves runs the drag
  // handler as the pointer goes, one call at a time with the latest place,
  // and `ondrop` when it is let go; without it, dragging moves the view.
  import { untrack } from "svelte";
  import { pointIn, scrolled, svgOf, zoomed, type Shape, type View } from "../drawing";
  import type { ShownText } from "../format";
  import type { Pointer } from "../live";
  import { pictureUrl } from "../pictures.svelte";

  let {
    drawing,
    units,
    shown,
    id,
    zoom = false,
    onpoint = null,
    ondrag = null,
    ondrop = null,
  }: {
    drawing: Shape[];
    units: { width: number; height: number };
    shown: (text: ShownText) => string;
    id?: string;
    /** Whether the person zooms and scrolls the drawing. */
    zoom?: boolean;
    /** A click on the drawing, in the canvas's units; none for a click beside it. */
    onpoint?: ((point: Pointer) => void) | null;
    /** The pointer moving with its button down, and where it was pressed; done when the handler is. */
    ondrag?: ((point: Pointer) => Promise<unknown>) | null;
    /** The pointer let go after a drag. */
    ondrop?: ((point: Pointer) => void) | null;
  } = $props();

  const elements = $derived(drawing.map((shape) => svgOf(shape, units, shown, pictureUrl)));

  /** The units shown; the whole canvas until the person zooms. */
  let view = $state<View>([0, 0, 1, 1]);
  // Only a canvas of other units starts over: a redraw keeps the view.
  const size = $derived(`${units.width} ${units.height}`);
  $effect.pre(() => {
    void size;
    view = untrack(() => [0, 0, units.width, units.height]);
  });
  const zoomedIn = $derived(view[2] < units.width || view[3] < units.height);

  let svg: SVGSVGElement | undefined = $state();

  /** A pixel moved this far is a drag, not a click. */
  const STILL = 4;

  /** The pointer that went down on the canvas, and where. */
  let pressed: { id: number; at: { x: number; y: number }; from: { x: number; y: number } | null; moved: boolean; viewAt: View } | null = null;
  /** The latest drag not sent yet, while the handler runs with the one before. */
  let waiting: Pointer | null = null;
  let sending: Promise<unknown> | null = null;

  function partSize() {
    const box = svg!.getBoundingClientRect();
    return { box, part: { width: box.width, height: box.height } };
  }

  function offset(e: { clientX: number; clientY: number }) {
    const { box } = partSize();
    return { x: e.clientX - box.left, y: e.clientY - box.top };
  }

  /** The units per pixel and where the drawing starts in the part, for the view shown. */
  function fit() {
    const { part } = partSize();
    const scale = Math.min(part.width / view[2], part.height / view[3]);
    return { scale, left: (part.width - view[2] * scale) / 2, top: (part.height - view[3] * scale) / 2 };
  }

  /** Where a pixel is in units, not rounded; beside the drawing too. */
  function exact(at: { x: number; y: number }) {
    const { scale, left, top } = fit();
    return { x: view[0] + (at.x - left) / scale, y: view[1] + (at.y - top) / scale };
  }

  /** Where a pointer that is down is, in whole units, at the nearest edge when beside the drawing. */
  function held(at: { x: number; y: number }) {
    const p = exact(at);
    const edge = (v: number, from: number, size: number) => Math.min(Math.max(Math.floor(v), Math.floor(from)), Math.ceil(from + size) - 1);
    return { x: edge(p.x, view[0], view[2]), y: edge(p.y, view[1], view[3]) };
  }

  function down(e: PointerEvent) {
    if (!e.isPrimary || e.button !== 0 || !(onpoint || ondrag || zoom)) return;
    const at = offset(e);
    pressed = { id: e.pointerId, at, from: pointIn(view, partSize().part, at), moved: false, viewAt: view };
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!pressed || pressed.id !== e.pointerId) return;
    const at = offset(e);
    if (!pressed.moved && Math.hypot(at.x - pressed.at.x, at.y - pressed.at.y) < STILL) return;
    pressed.moved = true;
    if (ondrag && pressed.from) {
      const now = held(at);
      drag({ x: now.x, y: now.y, fromX: pressed.from.x, fromY: pressed.from.y });
    } else if (zoom) {
      // Without a drag handler, dragging moves the view with the pointer.
      const { scale } = fit();
      view = scrolled(units, pressed.viewAt, { x: (pressed.at.x - at.x) / scale, y: (pressed.at.y - at.y) / scale });
    }
  }

  /** Sends a drag, or keeps it as the next while one is running: only the latest place counts. */
  function drag(point: Pointer) {
    if (sending) {
      waiting = point;
      return;
    }
    sending = ondrag!(point).finally(() => {
      sending = null;
      const next = waiting;
      waiting = null;
      if (next) drag(next);
    });
  }

  async function up(e: PointerEvent) {
    if (!pressed || pressed.id !== e.pointerId) return;
    const was = pressed;
    pressed = null;
    const at = offset(e);
    if (!was.moved) {
      const point = pointIn(view, partSize().part, at);
      if (onpoint && point) onpoint(point);
      return;
    }
    if (ondrag && was.from) {
      // The drop comes after the drags the handler is still running.
      while (sending) await sending;
      const now = held(at);
      ondrop?.({ x: now.x, y: now.y, fromX: was.from.x, fromY: was.from.y });
    }
  }

  function wheel(e: WheelEvent) {
    if (!zoom) return;
    e.preventDefault();
    const pixels = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
    if (e.ctrlKey || e.metaKey) {
      view = zoomed(units, view, Math.exp((-e.deltaY * pixels) / 300), exact(offset(e)));
    } else {
      const { scale } = fit();
      const across = e.shiftKey ? e.deltaY : e.deltaX;
      const down = e.shiftKey ? 0 : e.deltaY;
      view = scrolled(units, view, { x: (across * pixels) / scale, y: (down * pixels) / scale });
    }
  }

  // The wheel is taken from the page only while the canvas zooms.
  $effect(() => {
    if (!svg || !zoom) return;
    svg.addEventListener("wheel", wheel, { passive: false });
    return () => svg!.removeEventListener("wheel", wheel);
  });
</script>

<div class="canvas-frame">
  <svg
    {id}
    bind:this={svg}
    class="canvas-view"
    class:clickable={onpoint !== null}
    class:draggable={ondrag !== null || zoom}
    viewBox="{view[0]} {view[1]} {view[2]} {view[3]}"
    preserveAspectRatio="xMidYMid meet"
    role="img"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (pressed = null)}
  >
    {#each elements as element, i (i)}
      <svelte:element this={element.tag} {...element.attrs}>{element.text ?? ""}</svelte:element>
    {/each}
  </svg>
  {#if zoom && zoomedIn}
    <!-- Where the view is in the whole canvas. -->
    <div class="bar across" aria-hidden="true">
      <span style:left="{(view[0] / units.width) * 100}%" style:width="{(view[2] / units.width) * 100}%"></span>
    </div>
    <div class="bar down" aria-hidden="true">
      <span style:top="{(view[1] / units.height) * 100}%" style:height="{(view[3] / units.height) * 100}%"></span>
    </div>
  {/if}
</div>

<style>
  .canvas-frame {
    position: relative;
    width: 100%;
    height: 100%;
  }

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

  .canvas-view.draggable {
    touch-action: none;
  }

  .bar {
    position: absolute;
    pointer-events: none;
  }

  .bar.across {
    left: 2px;
    right: 10px;
    bottom: 2px;
    height: 6px;
  }

  .bar.down {
    top: 2px;
    bottom: 10px;
    right: 2px;
    width: 6px;
  }

  .bar span {
    position: absolute;
    border-radius: 3px;
    background: var(--text-muted);
    opacity: 0.5;
  }

  .bar.across span {
    top: 0;
    bottom: 0;
  }

  .bar.down span {
    left: 0;
    right: 0;
  }
</style>
