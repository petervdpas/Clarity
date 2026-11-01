<script lang="ts">
  // A divider that resizes what is next to it. It reports how far it was
  // dragged; the owner decides what that changes and keeps the size within
  // its limits. `x` resizes widths (a vertical bar), `y` heights.

  let {
    axis,
    onmove,
    label,
  }: {
    axis: "x" | "y";
    /** The distance dragged since the last call, in pixels. */
    onmove: (delta: number) => void;
    /** What the divider resizes, for assistive technology. */
    label: string;
  } = $props();

  let last: number | null = null;

  function down(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    last = axis === "x" ? e.clientX : e.clientY;
  }

  function move(e: PointerEvent) {
    if (last === null) return;
    const now = axis === "x" ? e.clientX : e.clientY;
    if (now !== last) onmove(now - last);
    last = now;
  }

  function up(e: PointerEvent) {
    last = null;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div
  class="splitter {axis}"
  role="separator"
  aria-label={label}
  aria-orientation={axis === "x" ? "vertical" : "horizontal"}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
></div>

<style>
  .splitter {
    flex: none;
    position: relative;
    z-index: 4;
    background: transparent;
    transition: background 120ms;
  }

  .splitter:hover,
  .splitter:active {
    background: var(--accent);
  }

  .splitter.x {
    width: 5px;
    margin: 0 -2px;
    cursor: col-resize;
  }

  .splitter.y {
    height: 5px;
    margin: -2px 0;
    cursor: row-resize;
  }
</style>
