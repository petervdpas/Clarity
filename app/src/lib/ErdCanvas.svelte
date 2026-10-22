<script lang="ts" module>
  /** What is picked on the canvas: an entity by name, or a relation by its place in the list. */
  export type ErdPick = { kind: "entity"; name: string } | { kind: "relation"; index: number } | null;
</script>

<script lang="ts">
  // An ERD drawn by Clarity itself, for designing: drag an Entity from the
  // palette onto the canvas, move boxes where you want them, and with the
  // Relation tool drag from one box to another to relate them. Ctrl+wheel
  // zooms, dragging the background pans. Proposals color the boxes they
  // are about: green for added, amber for changed, faded for removed.

  import type { Erd } from "./generated/Erd";
  import type { Mark } from "./diagramCanvas";
  import { addEntity, boxes, columns, connect, endMark, meeting, moveEntity, pinned, ROW, TITLE, type Box, type Point } from "./erdDesign";

  let {
    erd,
    marks = {},
    readonly = false,
    picked = $bindable(null),
    onchange: change = () => {},
    onopen = () => {},
  }: {
    erd: Erd;
    marks?: Record<string, Mark>;
    /** Drawn to look at: a click opens what a box draws. */
    readonly?: boolean;
    picked?: ErdPick;
    /** A change made on the canvas, written at once. */
    onchange?: (erd: Erd) => void;
    /** Opens what the box of `name` draws (a click when read only, a double click otherwise). */
    onopen?: (name: string) => void;
  } = $props();

  /** Hands a change on, with every box pinned where it is shown. */
  function onchange(next: Erd) {
    change(pinned(next, laid));
  }

  const FONT_SIZE = 12;
  const ZOOM_MIN = 0.25;
  const ZOOM_MAX = 3;
  const DRAG_SLOP = 4;

  // Text is measured in the IDE's monospace font, as it is drawn.
  const font = `${FONT_SIZE}px ${getComputedStyle(document.documentElement).getPropertyValue("--mono") || "monospace"}`;
  const context = document.createElement("canvas").getContext("2d");
  if (context) context.font = font;
  const measure = (text: string) => (context ? context.measureText(text).width : text.length * 7.2);

  let svg: SVGSVGElement | undefined = $state();
  let zoom = $state(1);
  let pan = $state<Point>({ x: 0, y: 0 });
  let tool = $state<"select" | "relation">("select");

  /** A box being moved, shown where the pointer has it until it is let go. */
  let moving = $state<{ name: string; x: number; y: number } | null>(null);
  /** A relation being drawn, from a box to the pointer. */
  let drawing = $state<{ from: string; to: Point } | null>(null);
  /** An entity being dragged from the palette, at the pointer on the screen. */
  let placing = $state<{ x: number; y: number } | null>(null);

  // Boxes stand where the diagram has them; only the one being moved follows the pointer.
  const laid = $derived(boxes(erd, measure).map((b) => (moving && b.name === moving.name ? { ...b, x: moving.x, y: moving.y } : b)));
  const byName = $derived(new Map(laid.map((b) => [b.name, b])));

  /** The point of the drawing under a point of the screen. */
  function toDrawing(clientX: number, clientY: number): Point {
    const r = svg?.getBoundingClientRect();
    return { x: (clientX - (r?.left ?? 0) - pan.x) / zoom, y: (clientY - (r?.top ?? 0) - pan.y) / zoom };
  }

  function boxAt(p: Point): Box | undefined {
    return laid.findLast((b) => p.x >= b.x && p.x <= b.x + b.w && p.y >= b.y && p.y <= b.y + b.h);
  }

  function setZoom(next: number, around?: Point) {
    const z = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, next));
    const r = svg?.getBoundingClientRect();
    const c = around ?? { x: (r?.width ?? 0) / 2, y: (r?.height ?? 0) / 2 };
    pan = { x: c.x - ((c.x - pan.x) * z) / zoom, y: c.y - ((c.y - pan.y) * z) / zoom };
    zoom = z;
  }

  function wheel(e: WheelEvent) {
    if (!(e.ctrlKey || e.metaKey)) {
      pan = { x: pan.x - e.deltaX, y: pan.y - e.deltaY };
      e.preventDefault();
      return;
    }
    e.preventDefault();
    const r = svg!.getBoundingClientRect();
    setZoom(zoom * (e.deltaY < 0 ? 1.15 : 1 / 1.15), { x: e.clientX - r.left, y: e.clientY - r.top });
  }

  /** Follows one press of the pointer to its end, as a click or a drag. */
  function follow(e: PointerEvent, onmove: (dx: number, dy: number, ev: PointerEvent) => void, onend: (moved: boolean, ev: PointerEvent) => void) {
    const start = { x: e.clientX, y: e.clientY };
    let moved = false;
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - start.x;
      const dy = ev.clientY - start.y;
      if (!moved && Math.hypot(dx, dy) < DRAG_SLOP) return;
      moved = true;
      onmove(dx, dy, ev);
    };
    const up = (ev: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      onend(moved, ev);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function pressBox(e: PointerEvent, box: Box) {
    if (e.button !== 0) return;
    e.stopPropagation();
    if (readonly) {
      follow(e, () => {}, (moved) => !moved && onopen(box.name));
      return;
    }
    if (tool === "relation") {
      drawing = { from: box.name, to: toDrawing(e.clientX, e.clientY) };
      follow(
        e,
        (_dx, _dy, ev) => drawing && (drawing = { ...drawing, to: toDrawing(ev.clientX, ev.clientY) }),
        (_moved, ev) => {
          const target = boxAt(toDrawing(ev.clientX, ev.clientY));
          const from = drawing?.from;
          drawing = null;
          if (from && target) {
            const next = connect(erd, from, target.name);
            if (next !== erd) {
              onchange(next);
              picked = { kind: "relation", index: next.relations.length - 1 };
            }
          }
        },
      );
      return;
    }
    picked = { kind: "entity", name: box.name };
    const origin = { x: box.x, y: box.y };
    follow(
      e,
      (dx, dy) => (moving = { name: box.name, x: origin.x + dx / zoom, y: origin.y + dy / zoom }),
      (moved) => {
        const last = moving;
        moving = null;
        if (moved && last) onchange(moveEntity(erd, last.name, last.x, last.y));
      },
    );
  }

  function pressRelation(e: PointerEvent, index: number) {
    if (e.button !== 0 || readonly) return;
    e.stopPropagation();
    picked = { kind: "relation", index };
  }

  function pressBackground(e: PointerEvent) {
    if (e.button !== 0) return;
    const origin = { ...pan };
    follow(
      e,
      (dx, dy) => (pan = { x: origin.x + dx, y: origin.y + dy }),
      (moved) => {
        if (!moved) picked = null;
      },
    );
  }

  /** Drags a new entity from the palette; let go over the canvas, it lands there. A click puts it in the middle. */
  function pressPalette(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    placing = { x: e.clientX, y: e.clientY };
    follow(
      e,
      (_dx, _dy, ev) => (placing = { x: ev.clientX, y: ev.clientY }),
      (moved, ev) => {
        placing = null;
        const r = svg?.getBoundingClientRect();
        if (!r) return;
        const inside = ev.clientX >= r.left && ev.clientX <= r.right && ev.clientY >= r.top && ev.clientY <= r.bottom;
        const at = moved ? (inside ? toDrawing(ev.clientX, ev.clientY) : null) : toDrawing(r.left + r.width / 2 - 60, r.top + r.height / 2 - 20);
        if (!at) return;
        const next = addEntity(erd, at.x, at.y);
        onchange(next);
        picked = { kind: "entity", name: next.entities[next.entities.length - 1].name };
      },
    );
  }

  /** The line of a relation, with the marks at its ends. */
  function line(from: Box, to: Box) {
    const [a, b] = meeting(from, to);
    const length = Math.hypot(b.x - a.x, b.y - a.y) || 1;
    const u = { x: (b.x - a.x) / length, y: (b.y - a.y) / length };
    return { a, b, u, mid: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } };
  }
</script>

<div class="erd" class:readonly>
  {#if !readonly}
    <div class="palette" role="toolbar" aria-label="Palette">
      <button class="item" onpointerdown={pressPalette} title="Drag onto the canvas to add an entity; a click puts it in the middle">
        <span class="glyph entity"></span> Entity
      </button>
      <button class="item" class:active={tool === "relation"} onclick={() => (tool = tool === "relation" ? "select" : "relation")} title="Drag from one entity to another to relate them: many of the first refer to one of the second">
        <span class="glyph relation"></span> Relation
      </button>
    </div>
  {/if}
  <div class="zoom">
    <button onclick={() => setZoom(zoom / 1.2)} aria-label="Zoom out" title="Zoom out (Ctrl+wheel)">-</button>
    <button class="level" onclick={() => ((zoom = 1), (pan = { x: 0, y: 0 }))} title="Back to 100%">{Math.round(zoom * 100)}%</button>
    <button onclick={() => setZoom(zoom * 1.2)} aria-label="Zoom in" title="Zoom in (Ctrl+wheel)">+</button>
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <svg bind:this={svg} class:connecting={tool === "relation"} onpointerdown={pressBackground} onwheel={wheel} style:font>
    <defs>
      <pattern id="erd-grid" width="20" height="20" patternUnits="userSpaceOnUse" patternTransform="translate({pan.x} {pan.y}) scale({zoom})">
        <circle cx="1" cy="1" r="1" class="dot" />
      </pattern>
    </defs>
    <rect class="ground" width="100%" height="100%" fill="url(#erd-grid)" />
    <g transform="translate({pan.x} {pan.y}) scale({zoom})">
      {#each erd.relations as r, i (i)}
        {@const from = byName.get(r.left)}
        {@const to = byName.get(r.right)}
        {#if from && to && from !== to}
          {@const l = line(from, to)}
          {@const start = endMark(l.a, l.u, r.leftCount)}
          {@const end = endMark(l.b, { x: -l.u.x, y: -l.u.y }, r.rightCount)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="relation" class:picked={picked?.kind === "relation" && picked.index === i} onpointerdown={(e) => pressRelation(e, i)}>
            <line class="hit" x1={l.a.x} y1={l.a.y} x2={l.b.x} y2={l.b.y} />
            <line class="wire" class:dashed={!r.identifying} x1={l.a.x} y1={l.a.y} x2={l.b.x} y2={l.b.y} />
            <path class="wire" d={start.path + end.path} />
            {#if start.ring}<circle class="ring" cx={start.ringAt.x} cy={start.ringAt.y} r="4" />{/if}
            {#if end.ring}<circle class="ring" cx={end.ringAt.x} cy={end.ringAt.y} r="4" />{/if}
            {#if r.label}<text class="label" x={l.mid.x} y={l.mid.y - 6} text-anchor="middle">{r.label}</text>{/if}
          </g>
        {/if}
      {/each}

      {#each laid as box (box.name)}
        {@const cols = columns(box, measure)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g
          class="box mark-{marks[box.name] ?? 'none'}"
          class:picked={picked?.kind === "entity" && picked.name === box.name}
          onpointerdown={(e) => pressBox(e, box)}
          ondblclick={() => !readonly && onopen(box.name)}
        >
          <rect class="body" x={box.x} y={box.y} width={box.w} height={box.h} rx="5" />
          <path class="title" d="M{box.x},{box.y + TITLE}V{box.y + 5}a5,5 0 0 1 5,-5H{box.x + box.w - 5}a5,5 0 0 1 5,5V{box.y + TITLE}Z" />
          <text class="name" x={box.x + box.w / 2} y={box.y + TITLE / 2 + 4} text-anchor="middle">{box.name}</text>
          {#each box.rows as row, i (i)}
            {@const y = box.y + TITLE + ROW * i + ROW / 2 + 4}
            {#if i % 2 === 1}<rect class="stripe" x={box.x + 1} y={box.y + TITLE + ROW * i} width={box.w - 2} height={ROW} />{/if}
            <text class="type" x={cols.type} {y}>{row.type}</text>
            <text class="field" x={cols.name} {y}>{row.name}</text>
            <text class="keys" x={cols.keys} {y}>{row.keys}</text>
          {/each}
        </g>
      {/each}

      {#if drawing}
        {@const from = byName.get(drawing.from)}
        {#if from}
          <line class="wire drawing" x1={from.x + from.w / 2} y1={from.y + from.h / 2} x2={drawing.to.x} y2={drawing.to.y} />
        {/if}
      {/if}
    </g>
  </svg>

  {#if placing}
    <div class="ghost" style:left="{placing.x}px" style:top="{placing.y}px">Entity</div>
  {/if}
  {#if erd.entities.length === 0}
    <p class="hint">{readonly ? "No entities yet." : "Drag an Entity from the palette onto the canvas to start."}</p>
  {/if}
</div>

<style>
  .erd {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    background: var(--bg);
  }

  svg {
    flex: 1;
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
    touch-action: none;
    user-select: none;
  }

  svg.connecting .box {
    cursor: crosshair;
  }

  .ground {
    pointer-events: all;
  }

  .dot {
    fill: var(--border);
  }

  .box {
    cursor: move;
  }

  .readonly .box {
    cursor: pointer;
  }

  .body {
    fill: var(--panel);
    stroke: var(--border-strong);
    stroke-width: 1;
  }

  .title {
    fill: var(--accent-soft);
    stroke: var(--border-strong);
    stroke-width: 1;
  }

  .stripe {
    fill: var(--panel-alt);
  }

  .box.picked .body,
  .box.picked .title,
  .box:hover .body {
    stroke: var(--accent);
    stroke-width: 2;
  }

  .box.mark-add .body,
  .box.mark-add .title {
    stroke: #2f8f4e;
    stroke-width: 2.5;
  }

  .box.mark-change .body,
  .box.mark-change .title {
    stroke: #c08a12;
    stroke-width: 2.5;
  }

  .box.mark-remove {
    opacity: 0.5;
  }

  text {
    fill: var(--text);
    font: inherit;
  }

  .name {
    font-weight: 700;
  }

  .type {
    fill: var(--code-type);
  }

  .keys {
    fill: var(--text-muted);
  }

  .wire {
    stroke: var(--text-muted);
    stroke-width: 1.5;
    fill: none;
  }

  .wire.dashed {
    stroke-dasharray: 6 4;
  }

  .wire.drawing {
    stroke: var(--accent);
    stroke-dasharray: 4 3;
    pointer-events: none;
  }

  .ring {
    fill: var(--bg);
    stroke: var(--text-muted);
    stroke-width: 1.5;
  }

  .hit {
    stroke: transparent;
    stroke-width: 12;
    cursor: pointer;
  }

  .relation.picked .wire,
  .relation:hover .wire {
    stroke: var(--accent);
  }

  .label {
    fill: var(--text-muted);
    font-style: italic;
  }

  .palette {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: flex-start;
    font-size: 12px;
    cursor: grab;
  }

  .item.active {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  .glyph {
    display: inline-block;
    width: 18px;
    height: 12px;
  }

  .glyph.entity {
    border: 1.5px solid var(--text-muted);
    border-top-width: 4px;
    border-radius: 2px;
  }

  .glyph.relation {
    border-bottom: 1.5px solid var(--text-muted);
    height: 7px;
  }

  .zoom {
    position: absolute;
    top: 10px;
    right: 12px;
    z-index: 1;
    display: flex;
    gap: 2px;
  }

  .zoom button {
    min-width: 28px;
    padding: 2px 6px;
    font-size: 12px;
  }

  .zoom .level {
    min-width: 52px;
    font-variant-numeric: tabular-nums;
  }

  .ghost {
    position: fixed;
    z-index: 60;
    transform: translate(-50%, -50%);
    padding: 6px 14px;
    background: var(--panel);
    border: 1.5px dashed var(--accent);
    border-radius: 5px;
    font-size: 12px;
    pointer-events: none;
  }

  .hint {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    margin: 0;
    color: var(--text-muted);
    font-size: 13px;
    pointer-events: none;
  }
</style>
