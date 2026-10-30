<script lang="ts">
  // The diagram designer's canvas, Clarity's own, for every kind it
  // designs: drag a box from the palette onto the canvas, move boxes where
  // you want them, and with a line tool drag from one box to another.
  // Ctrl+wheel zooms, dragging the background pans, and a right click on a
  // box's header opens what it draws. Proposals color the boxes they are
  // about: green for added, amber for changed, faded for removed. What the
  // boxes and lines are, and what an edit makes, is the kind's own
  // (erdDesign.ts, classDesign.ts); the drawing is designCanvas.ts.

  import ContextMenu from "./ContextMenu.svelte";
  import type { Mark } from "./diagramCanvas";
  import { boxes, columns, endMark, ends, FRAME_LABEL, frames, ROW, umlMark, type Box, type Frame, type CanvasEdit, type CanvasPick, type EndStyle, type Link, type PaletteItem, type Point, type Shape } from "./designCanvas";

  let {
    shapes,
    links,
    palette = [],
    marks = {},
    readonly = false,
    picked = $bindable(null),
    onedit = () => undefined,
    onopen = () => {},
    canopen = () => true,
    openLabel = () => "Open",
  }: {
    shapes: Shape[];
    links: Link[];
    /** What can be dragged onto the canvas or drawn with; none when read only. */
    palette?: PaletteItem[];
    marks?: Record<string, Mark>;
    /** Drawn to look at: boxes move, nothing else changes. */
    readonly?: boolean;
    picked?: CanvasPick;
    /** An edit made on the canvas, with the boxes as shown; gives what to pick after it. */
    onedit?: (edit: CanvasEdit, shown: Box[]) => CanvasPick | undefined;
    /** Opens what the box of `name` draws, from the menu of its header. */
    onopen?: (name: string) => void;
    /** Whether the box of `name` draws something that opens. */
    canopen?: (name: string) => boolean;
    /** The menu item that opens the box of `name`: "Open entity". */
    openLabel?: (name: string) => string;
  } = $props();

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
  /** The line tool in use, or null to pick and move. */
  let tool = $state<string | null>(null);

  /** A box being moved, shown where the pointer has it until it is let go. */
  let moving = $state<{ name: string; x: number; y: number } | null>(null);
  /** A frame being dragged: its boxes follow the pointer by (dx, dy) until it is let go. */
  let shifting = $state<{ group: string; dx: number; dy: number } | null>(null);
  /** A line being drawn, from a box to the pointer. */
  let drawing = $state<{ from: string; to: Point } | null>(null);
  /** An item being dragged from the palette, at the pointer on the screen. */
  let placing = $state<{ item: PaletteItem; x: number; y: number } | null>(null);
  /** The menu of a box's header, at the pointer on the screen. */
  let menu = $state<{ name: string; x: number; y: number } | null>(null);
  /** A line whose bend points are being dragged: shown so until it is let go. */
  let bending = $state<{ index: number; via: Point[] } | null>(null);

  /** The lines as drawn: one being bent has the points the pointer gives it. */
  const shownLinks = $derived(links.map((l, i) => (bending && bending.index === i ? { ...l, via: bending.via } : l)));

  // Boxes stand where the diagram has them; only the one being moved follows the pointer.
  const laid = $derived(
    boxes(shapes, measure, links).map((b) => {
      if (moving && b.name === moving.name) return { ...b, x: moving.x, y: moving.y };
      if (shifting && b.group === shifting.group) return { ...b, x: b.x + shifting.dx, y: b.y + shifting.dy };
      return b;
    }),
  );
  const framed = $derived(frames(laid));
  const byName = $derived(new Map(laid.map((b) => [b.name, b])));
  const meetings = $derived(ends(laid, shownLinks));

  /** Lays every box out again in layers, as boxes without a place are, in one edit. */
  function arrange() {
    const fresh = boxes(shapes.map((s) => ({ ...s, x: null, y: null })), measure, links);
    edit({ op: "moveAll", moves: fresh.map((b) => ({ name: b.name, x: b.x, y: b.y })) });
  }

  function edit(change: CanvasEdit) {
    const pick = onedit(change, laid);
    if (pick) picked = pick;
  }

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
    e.preventDefault();
    if (!(e.ctrlKey || e.metaKey)) {
      pan = { x: pan.x - e.deltaX, y: pan.y - e.deltaY };
      return;
    }
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
    menu = null;
    if (tool && !readonly) {
      const using = tool;
      drawing = { from: box.name, to: toDrawing(e.clientX, e.clientY) };
      follow(
        e,
        (_dx, _dy, ev) => drawing && (drawing = { ...drawing, to: toDrawing(ev.clientX, ev.clientY) }),
        (_moved, ev) => {
          const target = boxAt(toDrawing(ev.clientX, ev.clientY));
          const from = drawing?.from;
          drawing = null;
          if (from && target && target.name !== from) edit({ op: "connect", tool: using, from, to: target.name });
        },
      );
      return;
    }
    picked = { kind: "box", name: box.name };
    const origin = { x: box.x, y: box.y };
    follow(
      e,
      (dx, dy) => (moving = { name: box.name, x: origin.x + dx / zoom, y: origin.y + dy / zoom }),
      (moved) => {
        const last = moving;
        moving = null;
        if (moved && last) edit({ op: "move", name: last.name, x: last.x, y: last.y });
      },
    );
  }

  /** Drags a frame by its name: every box in it moves along, as one edit. */
  function pressFrame(e: PointerEvent, frame: Frame) {
    if (e.button !== 0) return;
    e.stopPropagation();
    menu = null;
    follow(
      e,
      (dx, dy) => (shifting = { group: frame.name, dx: dx / zoom, dy: dy / zoom }),
      (moved) => {
        const moves = moved ? laid.filter((b) => b.group === frame.name).map((b) => ({ name: b.name, x: b.x, y: b.y })) : [];
        shifting = null;
        if (moves.length > 0) edit({ op: "moveAll", moves });
      },
    );
  }

  /** The menu of a box's header; a right click below the header shows none. */
  function boxMenu(e: MouseEvent, box: Box) {
    e.preventDefault();
    const p = toDrawing(e.clientX, e.clientY);
    menu = p.y <= box.y + box.title && canopen(box.name) ? { name: box.name, x: e.clientX, y: e.clientY } : null;
    picked = { kind: "box", name: box.name };
  }

  /**
   * A press on a line picks it. Dragged from one of its bend points, it moves
   * that point; dragged from anywhere else on it, it adds a point there and
   * moves that. Letting go keeps the line's points.
   */
  function pressLink(e: PointerEvent, index: number) {
    if (e.button !== 0 || readonly) return;
    e.stopPropagation();
    menu = null;
    picked = { kind: "link", index };
    const at = meetings[index];
    if (!at) return;
    const via = shownLinks[index].via ?? [];
    const p = toDrawing(e.clientX, e.clientY);
    const near = 8 / zoom;
    let point = via.findIndex((v) => Math.hypot(v.x - p.x, v.y - p.y) <= near);
    let points = [...via];
    if (point < 0) {
      // The segment pressed: the new point goes between its two ends.
      const all = [at[0], ...via, at[1]];
      let best = 0;
      let bestDistance = Infinity;
      for (let i = 0; i < all.length - 1; i++) {
        const d = distanceToSegment(p, all[i], all[i + 1]);
        if (d < bestDistance) [best, bestDistance] = [i, d];
      }
      point = best;
      points = [...via.slice(0, best), p, ...via.slice(best)];
    }
    const start = points[point];
    follow(
      e,
      (dx, dy) => {
        const moved = [...points];
        moved[point] = { x: start.x + dx / zoom, y: start.y + dy / zoom };
        bending = { index, via: moved };
      },
      (moved) => {
        const last = bending;
        bending = null;
        if (moved && last) edit({ op: "bend", index, via: last.via });
      },
    );
  }

  /** A double click on a bend point takes it away: the line goes straight there again. */
  function unbend(e: MouseEvent, index: number, point: number) {
    if (readonly) return;
    e.stopPropagation();
    const via = (shownLinks[index].via ?? []).filter((_, i) => i !== point);
    edit({ op: "bend", index, via });
  }

  function distanceToSegment(p: Point, a: Point, b: Point): number {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const length = dx * dx + dy * dy;
    const t = length === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / length));
    return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
  }

  function pressBackground(e: PointerEvent) {
    menu = null;
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

  /** Drags a box from the palette; let go over the canvas, it lands there. A click puts it in the middle. */
  function pressPalette(e: PointerEvent, item: PaletteItem) {
    if (e.button !== 0) return;
    e.preventDefault();
    placing = { item, x: e.clientX, y: e.clientY };
    follow(
      e,
      (_dx, _dy, ev) => (placing = { item, x: ev.clientX, y: ev.clientY }),
      (moved, ev) => {
        placing = null;
        const r = svg?.getBoundingClientRect();
        if (!r) return;
        const inside = ev.clientX >= r.left && ev.clientX <= r.right && ev.clientY >= r.top && ev.clientY <= r.bottom;
        const at = moved ? (inside ? toDrawing(ev.clientX, ev.clientY) : null) : toDrawing(r.left + r.width / 2 - 60, r.top + r.height / 2 - 20);
        if (at) edit({ op: "add", item: item.id, x: at.x, y: at.y });
      },
    );
  }

  /**
   * A line from end to end through its bend points: the points, the way it
   * leaves its first end, the way it comes into its last, and the middle of
   * its length, where its label goes.
   */
  function line([a, b]: [Point, Point], via: Point[] = []) {
    const points = [a, ...via, b];
    const unit = (from: Point, to: Point) => {
      const length = Math.hypot(to.x - from.x, to.y - from.y) || 1;
      return { x: (to.x - from.x) / length, y: (to.y - from.y) / length };
    };
    const lengths = points.slice(1).map((p, i) => Math.hypot(p.x - points[i].x, p.y - points[i].y));
    let half = lengths.reduce((sum, l) => sum + l, 0) / 2;
    let mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    for (let i = 0; i < lengths.length; i++) {
      if (half <= lengths[i] || i === lengths.length - 1) {
        const t = lengths[i] ? Math.min(1, half / lengths[i]) : 0;
        mid = { x: points[i].x + (points[i + 1].x - points[i].x) * t, y: points[i].y + (points[i + 1].y - points[i].y) * t };
        break;
      }
      half -= lengths[i];
    }
    return {
      points: points.map((p) => `${p.x},${p.y}`).join(" "),
      a,
      b,
      u: unit(a, points[1]),
      into: unit(b, points[points.length - 2]),
      mid,
    };
  }

  /** What is drawn at one end: an ERD's count, or a UML head with its multiplicity. */
  function endOf(at: Point, along: Point, style: EndStyle) {
    if (style.kind === "count") return { count: endMark(at, along, style.count), uml: null, text: "" };
    return { count: null, uml: umlMark(at, along, style.head), text: style.text };
  }
</script>

<div class="canvas" class:readonly>
  {#if !readonly && palette.length > 0}
    <div class="palette" role="toolbar" aria-label="Palette">
      {#each palette as item (item.id)}
        {#if item.tool}
          <button class="item" class:active={tool === item.id} onclick={() => (tool = tool === item.id ? null : item.id)} title={item.title}>
            <span class="glyph {item.glyph}"></span> {item.label}
          </button>
        {:else}
          <button class="item" onpointerdown={(e) => pressPalette(e, item)} title={item.title}>
            <span class="glyph {item.glyph}"></span> {item.label}
          </button>
        {/if}
      {/each}
    </div>
  {/if}
  <div class="zoom">
    <button onclick={() => setZoom(zoom / 1.2)} aria-label="Zoom out" title="Zoom out (Ctrl+wheel)">-</button>
    <button class="level" onclick={() => ((zoom = 1), (pan = { x: 0, y: 0 }))} title="Back to 100%">{Math.round(zoom * 100)}%</button>
    <button onclick={() => setZoom(zoom * 1.2)} aria-label="Zoom in" title="Zoom in (Ctrl+wheel)">+</button>
    {#if !readonly}
      <button onclick={arrange} title="Lay every box out again: what is referred to above what refers to it">Arrange</button>
    {/if}
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <svg bind:this={svg} class:connecting={tool !== null} onpointerdown={pressBackground} onwheel={wheel} style:font>
    <defs>
      <pattern id="design-grid" width="20" height="20" patternUnits="userSpaceOnUse" patternTransform="translate({pan.x} {pan.y}) scale({zoom})">
        <circle cx="1" cy="1" r="1" class="dot" />
      </pattern>
    </defs>
    <rect class="ground" width="100%" height="100%" fill="url(#design-grid)" />
    <g transform="translate({pan.x} {pan.y}) scale({zoom})">
      {#each framed as frame (frame.name)}
        <rect class="frame" x={frame.x} y={frame.y} width={frame.w} height={frame.h} rx="8" />
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g class="frame-name" onpointerdown={(e) => pressFrame(e, frame)}>
          <rect x={frame.x} y={frame.y} width={frame.w} height={FRAME_LABEL} rx="8" />
          <text x={frame.x + 10} y={frame.y + 14}>{frame.name}</text>
        </g>
      {/each}

      {#each links as link, i (i)}
        {@const at = meetings[i]}
        {#if at}
          {@const via = shownLinks[i].via ?? []}
          {@const l = line(at, via)}
          {@const start = endOf(l.a, l.u, link.leftEnd)}
          {@const end = endOf(l.b, l.into, link.rightEnd)}
          {@const isPicked = picked?.kind === "link" && picked.index === i}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <g class="link" class:picked={isPicked} onpointerdown={(e) => pressLink(e, i)}>
            <polyline class="hit" points={l.points} />
            <polyline class="wire" class:dashed={link.dashed} points={l.points} />
            {#each [start, end] as mark, m (m)}
              {#if mark.count}
                <path class="wire" d={mark.count.path} />
                {#if mark.count.ring}<circle class="ring" cx={mark.count.ringAt.x} cy={mark.count.ringAt.y} r="4" />{/if}
              {:else if mark.uml}
                {#if mark.uml.path}<path class="head" class:closed={mark.uml.closed} class:filled={mark.uml.filled} d={mark.uml.path} />{/if}
                {#if mark.text}<text class="count" x={mark.uml.textAt.x} y={mark.uml.textAt.y + 4} text-anchor="middle">{mark.text}</text>{/if}
              {/if}
            {/each}
            {#if link.label}<text class="label" x={l.mid.x} y={l.mid.y - 6} text-anchor="middle">{link.label}</text>{/if}
            {#if isPicked && !readonly}
              <!-- Its bend points: drag one to move it, double-click it to take it away. -->
              {#each via as point, p (p)}
                <circle class="bend" cx={point.x} cy={point.y} r={5 / zoom} ondblclick={(e) => unbend(e, i, p)} />
              {/each}
            {/if}
          </g>
        {/if}
      {/each}

      {#each laid as box (box.name)}
        {@const cols = columns(box, measure)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <g
          class="box tone-{box.tone} mark-{marks[box.name] ?? 'none'}"
          class:picked={picked?.kind === "box" && picked.name === box.name}
          onpointerdown={(e) => pressBox(e, box)}
          oncontextmenu={(e) => boxMenu(e, box)}
        >
          <rect class="body" x={box.x} y={box.y} width={box.w} height={box.h} rx="5" />
          <path class="title" d="M{box.x},{box.y + box.title}V{box.y + 5}a5,5 0 0 1 5,-5H{box.x + box.w - 5}a5,5 0 0 1 5,5V{box.y + box.title}Z" />
          {#if box.stereotype}
            <text class="stereotype" x={box.x + box.w / 2} y={box.y + 14} text-anchor="middle">«{box.stereotype}»</text>
          {/if}
          <text class="name" x={box.x + box.w / 2} y={box.y + box.title / 2 + 4 + (box.stereotype ? 5 : 0)} text-anchor="middle">{box.name}</text>
          {#each box.rows as row, r (r)}
            {@const y = box.y + box.title + ROW * r + ROW / 2 + 4}
            {#if r % 2 === 1}<rect class="stripe" x={box.x + 1} y={box.y + box.title + ROW * r} width={box.w - 2} height={ROW} />{/if}
            {#each row as cell, c (c)}
              <text class="cell {box.cells?.[c] ?? ''}" x={cols[c]} {y}>{cell}</text>
            {/each}
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

  {#if menu}
    {@const name = menu.name}
    <ContextMenu x={menu.x} y={menu.y} items={[{ label: openLabel(name), run: () => onopen(name) }]} onclose={() => (menu = null)} />
  {/if}
  {#if placing}
    <div class="ghost" style:left="{placing.x}px" style:top="{placing.y}px">{placing.item.label}</div>
  {/if}
  {#if shapes.length === 0}
    <p class="hint">{readonly ? "Nothing to draw yet." : "Drag a box from the palette onto the canvas to start."}</p>
  {/if}
</div>

<style>
  .canvas {
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

  .tone-enum .title {
    fill: color-mix(in srgb, #2f8f4e 18%, var(--panel));
  }

  .tone-engine .title {
    fill: color-mix(in srgb, #c08a12 18%, var(--panel));
  }

  .tone-module .title {
    fill: color-mix(in srgb, #7a5cc0 18%, var(--panel));
  }

  .frame {
    fill: color-mix(in srgb, var(--accent) 4%, transparent);
    stroke: var(--border-strong);
    stroke-dasharray: 5 4;
    pointer-events: none;
  }

  .frame-name {
    cursor: move;
  }

  .frame-name rect {
    fill: transparent;
  }

  .frame-name text {
    font-weight: 700;
    fill: var(--text-muted);
  }

  .frame-name:hover text {
    fill: var(--accent);
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

  .stereotype {
    font-size: 10px;
    fill: var(--text-muted);
  }

  .cell.type {
    fill: var(--code-type);
  }

  .cell.keys,
  .count {
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

  .head {
    stroke: var(--text-muted);
    stroke-width: 1.5;
    fill: none;
  }

  .head.closed {
    fill: var(--bg);
  }

  .head.filled {
    fill: var(--text-muted);
  }

  .ring {
    fill: var(--bg);
    stroke: var(--text-muted);
    stroke-width: 1.5;
  }

  .hit {
    stroke: transparent;
    stroke-width: 12;
    fill: none;
    cursor: pointer;
  }

  /* A picked line's bend points: drag one to move it, double-click it to take it away. */
  .bend {
    fill: var(--bg);
    stroke: var(--accent);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
    cursor: move;
  }

  .link.picked .wire,
  .link:hover .wire,
  .link.picked .head,
  .link:hover .head {
    stroke: var(--accent);
  }

  .link.picked .head.filled,
  .link:hover .head.filled {
    fill: var(--accent);
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
    position: relative;
    display: inline-block;
    width: 18px;
    height: 12px;
  }

  .glyph.entity,
  .glyph.enum,
  .glyph.engine,
  .glyph.module {
    border: 1.5px solid var(--text-muted);
    border-top-width: 4px;
    border-radius: 2px;
  }

  .glyph.enum {
    border-top-color: #2f8f4e;
  }

  .glyph.engine {
    border-top-color: #c08a12;
  }

  .glyph.module {
    border-top-color: #7a5cc0;
  }

  .glyph.relation,
  .glyph.association,
  .glyph.composition,
  .glyph.aggregation {
    border-bottom: 1.5px solid var(--text-muted);
    height: 7px;
  }

  /* The head of each kind of line, drawn small at the glyph's end. */
  .glyph.association::after {
    content: "";
    position: absolute;
    right: 0;
    bottom: -4px;
    width: 6px;
    height: 6px;
    border-top: 1.5px solid var(--text-muted);
    border-right: 1.5px solid var(--text-muted);
    transform: rotate(45deg);
  }

  .glyph.composition::before,
  .glyph.aggregation::before {
    content: "";
    position: absolute;
    left: 0;
    bottom: -4px;
    width: 6px;
    height: 6px;
    border: 1.5px solid var(--text-muted);
    background: var(--panel);
    transform: rotate(45deg);
  }

  .glyph.composition::before {
    background: var(--text-muted);
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
