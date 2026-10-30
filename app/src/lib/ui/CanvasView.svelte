<script lang="ts">
  // A canvas drawn by code: the shapes its draw handler sent, in SVG, in the
  // canvas's own units, fitted to the part in proportion and centred.
  // Avalonia's CanvasView draws the same (conformance/drawing.json). With
  // `onpoint`, a click (mouse, pen or touch) gives the unit under it.
  //
  // With `zoom` it moves as Clarity's own diagram designer does
  // (conformance/drawing.json, views): it starts whole; the wheel pans (with
  // Shift sideways), Ctrl or Cmd with the wheel zooms around the pointer, as
  // a touchpad's pinch does, and the buttons in its corner zoom out, back to
  // whole, and in. With `ondrag` a press that moves runs the drag handler as
  // the pointer goes, one call at a time with the latest place, and `ondrop`
  // when it is let go. A drag that moves nothing (the drawing stays the same
  // after the handler's first call), and any drag without `ondrag`, pans.
  // With `grid` it shows a dot every so many units over the whole view, after
  // the drawing's clear (conformance/drawing.json, grids).
  import { colorOf, gridEvery, pointIn, scrolled, svgOf, wholeView, zoomed, type Shape, type View } from "../drawing";
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
    background = null,
    dotSize = null,
    dotColor = null,
    words = { zoomOut: "Zoom out", zoomWhole: "Show it whole", zoomIn: "Zoom in" },
    onpoint = null,
    ondrag = null,
    ondrop = null,
  }: {
    drawing: Shape[];
    units: { width: number; height: number };
    shown: (text: ShownText) => string;
    id?: string;
    /** Whether the person zooms and pans the drawing. */
    zoom?: boolean;
    /** The units between the grid's dots; none for no grid. */
    grid?: number | null;
    /** What a clear without a color fills it with; empty or none is the theme's panel. */
    background?: string | null;
    /** How many pixels across the grid's dots are (2 without it), and their color (the theme's border without it). */
    dotSize?: number | null;
    dotColor?: string | null;
    /** Clarity's words for the zoom buttons, in the person's language. */
    words?: { zoomOut: string; zoomWhole: string; zoomIn: string };
    /** A click on the drawing, in the canvas's units; none for a click beside it. */
    onpoint?: ((point: Pointer) => void) | null;
    /** The pointer moving with its button down, and where it was pressed; done when the handler is. */
    ondrag?: ((point: Pointer) => Promise<unknown>) | null;
    /** The pointer let go after a drag. */
    ondrop?: ((point: Pointer) => void) | null;
  } = $props();

  // A clear without a color is the canvas's background.
  const drawn = $derived(
    drawing.map((shape) => svgOf(shape.op === "clear" && !shape.color ? { ...shape, color: background || "panel" } : shape, units, shown, pictureUrl)),
  );

  let svg: SVGSVGElement | undefined = $state();
  // The part's size: the whole canvas is fitted to it, and the view fills it.
  let partWidth = $state(0);
  let partHeight = $state(0);

  /** Where the person zoomed and panned to; none while the canvas shows whole. */
  let moved = $state<View | null>(null);
  // Only a canvas of other units starts over: a redraw keeps the view.
  const size = $derived(`${units.width} ${units.height}`);
  $effect.pre(() => {
    void size;
    moved = null;
  });
  const view = $derived<View>(zoom && moved ? moved : wholeView(units, { width: partWidth || 1, height: partHeight || 1 }));
  const viewBox = $derived(`${view[1]} ${view[2]} ${(partWidth || 1) / view[0]} ${(partHeight || 1) / view[0]}`);
  // A canvas that zooms is a sheet without edges: its clear fills the whole view.
  const elements = $derived(
    zoom
      ? drawn.map((element, i) =>
          drawing[i].op === "clear" ? { ...element, attrs: { ...element.attrs, x: view[1], y: view[2], width: (partWidth || 1) / view[0], height: (partHeight || 1) / view[0] } } : element,
        )
      : drawn,
  );

  /** The grid's dots: every `gridEvery` units, the same size whatever the zoom. */
  const dots = $derived(grid ? { every: gridEvery(grid, view[0]), r: Math.max(dotSize || 2, 1) / 2 / view[0], color: colorOf(dotColor || "border") } : null);
  /** The grid goes after the drawing's clear, so the shapes cover it; first when there is none. */
  const gridAfter = $derived(drawing.findIndex((shape) => shape.op === "clear"));
  // Each canvas's pattern has a name of its own in the page.
  const patternId = `canvas-grid-${Math.random().toString(36).slice(2)}`;

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

  /** The part's size as it is now, measured where the pointer is read. */
  function part() {
    const box = svg!.getBoundingClientRect();
    return { box, size: { width: box.width, height: box.height } };
  }

  function offset(e: { clientX: number; clientY: number }) {
    const { box } = part();
    return { x: e.clientX - box.left, y: e.clientY - box.top };
  }

  /** The view as the pointer finds it, for the part's size now. */
  function viewNow(): View {
    return zoom && moved ? moved : wholeView(units, part().size);
  }

  /** The unit under a pointer that is down, at the canvas's nearest edge beside it. */
  function held(at: { x: number; y: number }) {
    const v = viewNow();
    const edge = (u: number, end: number) => Math.min(Math.max(Math.floor(u), 0), Math.ceil(end) - 1);
    return { x: edge(v[1] + at.x / v[0], units.width), y: edge(v[2] + at.y / v[0], units.height) };
  }

  function down(e: PointerEvent) {
    if (!e.isPrimary || e.button !== 0 || !(onpoint || ondrag || zoom)) return;
    const at = offset(e);
    const viewAt = viewNow();
    pressed = { id: e.pointerId, at, from: pointIn(units, viewAt, at), moved: false, viewAt, moves: ondrag ? "unknown" : "view", now: at };
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
    moved = scrolled(pressed.viewAt, { x: pressed.at.x - pressed.now.x, y: pressed.at.y - pressed.now.y });
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
      const point = pointIn(units, viewNow(), at);
      if (onpoint && point) onpoint(point);
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

  /** Zooms by `factor` around a pixel of the part; its middle without one. */
  function zoomBy(factor: number, at?: { x: number; y: number }) {
    const size = part().size;
    moved = zoomed(units, size, viewNow(), factor, at ?? { x: size.width / 2, y: size.height / 2 });
  }

  function wheel(e: WheelEvent) {
    if (!zoom) return;
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      zoomBy(e.deltaY < 0 ? 1.15 : 1 / 1.15, offset(e));
      return;
    }
    const pixels = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
    const across = e.shiftKey ? e.deltaY : e.deltaX;
    const down = e.shiftKey ? 0 : e.deltaY;
    moved = scrolled(viewNow(), { x: across * pixels, y: down * pixels });
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
    viewBox={viewBox}
    preserveAspectRatio="xMinYMin meet"
    role="img"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={() => (pressed = null)}
  >
    {#if dots}
      <defs>
        <pattern id={patternId} width={dots.every} height={dots.every} patternUnits="userSpaceOnUse">
          <!-- A dot on each corner of the tile: the four quarters make whole dots. -->
          {#each [[0, 0], [dots.every, 0], [0, dots.every], [dots.every, dots.every]] as [cx, cy] (`${cx} ${cy}`)}
            <circle {cx} {cy} r={dots.r} fill={dots.color} />
          {/each}
        </pattern>
      </defs>
    {/if}
    {#if dots && gridAfter < 0}{@render gridShown()}{/if}
    {#each elements as element, i (i)}
      <svelte:element this={element.tag} {...element.attrs}>{element.text ?? ""}</svelte:element>
      {#if dots && i === gridAfter}{@render gridShown()}{/if}
    {/each}
  </svg>
  {#if zoom}
    <!-- As the designer's: out, back to whole (the zoom it is at), in. -->
    <div class="zoom">
      <button type="button" onclick={() => zoomBy(1 / 1.2)} title={words.zoomOut} aria-label={words.zoomOut}>-</button>
      <button type="button" class="level" onclick={() => (moved = null)} title={words.zoomWhole}>{Math.round(view[0] * 100)}%</button>
      <button type="button" onclick={() => zoomBy(1.2)} title={words.zoomIn} aria-label={words.zoomIn}>+</button>
    </div>
  {/if}
</div>

{#snippet gridShown()}
  <rect class="grid" x={view[1]} y={view[2]} width={(partWidth || 1) / view[0]} height={(partHeight || 1) / view[0]} fill="url(#{patternId})" />
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

  .zoom {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    gap: 2px;
  }

  .zoom button {
    min-width: 26px;
    height: 24px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--panel);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .zoom .level {
    min-width: 48px;
  }
</style>
