<script lang="ts">
  // A window as the designer and the preview show it: at the size it opens
  // at on every stack (its own `size`, or WINDOW_WIDTH by WINDOW_HEIGHT), so
  // its columns share the same width and its growing rows the same height as
  // when it runs. It is scaled down to fit the room it has, never up. The
  // children are its title bar and its client, as tall as the window. Given
  // `onresize`, as the designer does, its right edge, bottom edge and corner
  // drag to size it; the size shows while it is dragged.
  import type { Snippet } from "svelte";
  import { WINDOW_WIDTH, resized, type WindowEdge } from "./grid";

  let {
    area,
    clientWidth = WINDOW_WIDTH,
    clientHeight,
    onresize,
    children,
  }: {
    /** The theme's class and light or dark scheme. */
    area: { class: string; scheme: string };
    /** The window's width, without its border. */
    clientWidth?: number;
    /** The client's height, for sizing by dragging. */
    clientHeight?: number;
    /** Sizes the window by its edges: while dragging (`done` false), and once let go. */
    onresize?: (size: { width: number; height: number }, done: boolean) => void;
    children: Snippet;
  } = $props();

  let room = $state(0);
  let height = $state(0);
  // The window's border, one pixel on each side.
  const width = $derived(clientWidth + 2);
  const scale = $derived(room > 0 ? Math.min(1, room / width) : 1);

  // The drag under way: the edge, where it started and the size then.
  let drag = $state<{ edge: WindowEdge; x: number; y: number; start: { width: number; height: number }; scale: number; size: { width: number; height: number } } | null>(null);

  function begin(e: PointerEvent, edge: WindowEdge) {
    if (e.button !== 0 || !onresize || clientHeight === undefined) return;
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const start = { width: clientWidth, height: clientHeight };
    drag = { edge, x: e.clientX, y: e.clientY, start, scale, size: start };
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    drag.size = resized(drag.start, drag.edge, e.clientX - drag.x, e.clientY - drag.y, drag.scale);
    onresize?.(drag.size, false);
  }

  function end() {
    if (!drag) return;
    const size = drag.size;
    drag = null;
    onresize?.(size, true);
  }
</script>

<div class="room" bind:clientWidth={room}>
  <div class="fit" style:width="{width * scale}px" style:height="{height * scale}px">
    <div class="window {area.class}" data-scheme={area.scheme} style:width="{width}px" style:transform="scale({scale})" bind:offsetHeight={height}>
      {@render children()}
      {#if onresize}
        {#each ["right", "bottom", "corner"] as const as edge (edge)}
          <div
            class="edge {edge}"
            role="separator"
            aria-label="Drag to size the window"
            title="Drag to size the window"
            onpointerdown={(e) => begin(e, edge)}
            onpointermove={move}
            onpointerup={end}
            onpointercancel={end}
          ></div>
        {/each}
      {/if}
      {#if drag}
        <span class="size">{drag.size.width} x {drag.size.height}</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .room {
    min-width: 0;
  }

  .window {
    transform-origin: top left;
    background: var(--panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.08);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .edge {
    position: absolute;
    z-index: 5;
    touch-action: none;
  }

  .edge.right {
    top: 0;
    right: 0;
    bottom: 10px;
    width: 6px;
    cursor: ew-resize;
  }

  .edge.bottom {
    left: 0;
    right: 10px;
    bottom: 0;
    height: 6px;
    cursor: ns-resize;
  }

  .edge.corner {
    right: 0;
    bottom: 0;
    width: 12px;
    height: 12px;
    cursor: nwse-resize;
  }

  .edge:hover {
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .size {
    position: absolute;
    right: 16px;
    bottom: 16px;
    z-index: 6;
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid var(--accent);
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    pointer-events: none;
  }
</style>
