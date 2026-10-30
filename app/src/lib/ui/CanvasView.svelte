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
  // and `ondrop` when it is let go. A drag that moves nothing (the drawing
  // stays the same after the handler's first call), and any drag without
  // `ondrag`, moves the view instead, on a canvas that zooms. With `grid` it
  // shows a grid behind the shapes (after the drawing's clear), and with
  // `snap` every place it gives is the nearest point of that grid
  // (conformance/drawing.json, grids).
  import { untrack } from "svelte";
  import { colorOf, gridEvery, pointIn, scrolled, snapped, svgOf, zoomed, type Shape, type View } from "../drawing";
  import type { ShownText } from "../format";
  import type { Pointer } from "../live";
  import { pictureUrl } from "../pictures.svelte";

  let {
    drawing,
    units,
    shown,
    id,
    zoom = false,
    grid = null,
    snap = false,
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
    /** The units between the grid's lines; none for no grid. */
    grid?: number | null;
    /** Whether the places it gives snap to the grid. */
    snap?: boolean;
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

  // The part's size, for how far apart the grid's lines are shown.
  let partWidth = $state(0);
  let partHeight = $state(0);

  /** Where the grid's lines go, in the view: every `gridEvery` units, thinned as the view zooms out. */
  const gridLines = $derived.by(() => {
    if (!grid) return { across: [] as number[], down: [] as number[] };
    const scale = Math.min(partWidth / view[2], partHeight / view[3]);
    const every = gridEvery(grid, scale > 0 ? scale : 1);
    const along = (from: number, size: number, end: number) => {
      const out: number[] = [];
      for (let v = Math.ceil(from / every) * every; v <= Math.min(from + size, end); v += every) out.push(v);
      return out;
    };
    return { across: along(view[0], view[2], units.width), down: along(view[1], view[3], units.height) };
  });
  /** The grid goes after the drawing's clear, so the shapes cover it; first when there is none. */
  const gridAfter = $derived(drawing.findIndex((shape) => shape.op === "clear"));

  /** A place the canvas gives: on the grid when it snaps. */
  function placed<P extends { x: number; y: number }>(point: P): P {
    return snap && grid ? { ...point, ...snapped(units, grid, point) } : point;
  }

  /** A pixel moved this far is a drag, not a click. */
  const STILL = 4;

  /** The pointer that went down on the canvas, and where. */
  let pressed: {
    id: number;
    at: { x: number; y: number };
    from: { x: number; y: number } | null;
    moved: boolean;
    viewAt: View;
    /** What the drag does: unknown until the handler's first call says, then the handler's or the view's. */
    moves: "unknown" | "handler" | "view";
    /** Where the pointer is now, in pixels. */
    now: { x: number; y: number };
  } | null = null;
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
    return placed({ x: edge(p.x, view[0], view[2]), y: edge(p.y, view[1], view[3]) });
  }

  function down(e: PointerEvent) {
    if (!e.isPrimary || e.button !== 0 || !(onpoint || ondrag || zoom)) return;
    const at = offset(e);
    const from = pointIn(view, partSize().part, at);
    pressed = { id: e.pointerId, at, from: from && placed(from), moved: false, viewAt: view, moves: ondrag ? "unknown" : "view", now: at };
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!pressed || pressed.id !== e.pointerId) return;
    const at = offset(e);
    if (!pressed.moved && Math.hypot(at.x - pressed.at.x, at.y - pressed.at.y) < STILL) return;
    pressed.moved = true;
    pressed.now = at;
    if (pressed.moves !== "view" && ondrag && pressed.from) {
      const now = held(at);
      drag({ x: now.x, y: now.y, fromX: pressed.from.x, fromY: pressed.from.y });
    } else pan();
  }

  /** Moves the view with the pointer, from where it was pressed. */
  function pan() {
    if (!zoom || !pressed) return;
    const { scale } = fit();
    view = scrolled(units, pressed.viewAt, { x: (pressed.at.x - pressed.now.x) / scale, y: (pressed.at.y - pressed.now.y) / scale });
  }

  /** Sends a drag, or keeps it as the next while one is running: only the latest place counts. */
  function drag(point: Pointer) {
    if (sending) {
      waiting = point;
      return;
    }
    const before = JSON.stringify(drawing);
    const press = pressed;
    sending = ondrag!(point).finally(() => {
      sending = null;
      // The first call says what the drag does: a drawing that stayed the same moves the view.
      if (press && press.moves === "unknown") {
        press.moves = zoom && JSON.stringify(drawing) === before ? "view" : "handler";
        if (press.moves === "view") {
          waiting = null;
          if (press === pressed) pan();
        }
      }
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
      if (onpoint && point) onpoint(placed(point));
      return;
    }
    if (ondrag && was.from) {
      // The drop comes after the drags the handler is still running; a drag that moved the view has none.
      while (sending) await sending;
      if (was.moves === "view") return;
      const now = held(at);
      ondrop?.({ x: now.x, y: now.y, fromX: was.from.x, fromY: was.from.y });
    }
  }

  function wheel(e: WheelEvent) {
    if (!zoom) return;
    e.preventDefault();
    const pixels = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
    if (e.ctrlKey || e.metaKey) {
      view = zoomed(units, partSize().part, view, Math.exp((-e.deltaY * pixels) / 300), exact(offset(e)));
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

<div class="canvas-frame" bind:clientWidth={partWidth} bind:clientHeight={partHeight}>
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
    {#if grid && gridAfter < 0}{@render gridShown()}{/if}
    {#each elements as element, i (i)}
      <svelte:element this={element.tag} {...element.attrs}>{element.text ?? ""}</svelte:element>
      {#if grid && i === gridAfter}{@render gridShown()}{/if}
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

{#snippet gridShown()}
  <g class="grid" stroke={colorOf("border")} stroke-width="1" vector-effect="non-scaling-stroke">
    {#each gridLines.across as x (x)}
      <line x1={x} y1={view[1]} x2={x} y2={Math.min(view[1] + view[3], units.height)} vector-effect="non-scaling-stroke" />
    {/each}
    {#each gridLines.down as y (y)}
      <line x1={view[0]} y1={y} x2={Math.min(view[0] + view[2], units.width)} y2={y} vector-effect="non-scaling-stroke" />
    {/each}
  </g>
{/snippet}

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

  .grid {
    opacity: 0.5;
    pointer-events: none;
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
