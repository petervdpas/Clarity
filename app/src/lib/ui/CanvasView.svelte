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
  // the drawing's clear (conformance/drawing.json, grids). With `onbend` its
  // links bend: a press on one adds a point there or takes the point under it,
  // dragging moves that point, a double click on a point takes it away, and
  // letting go gives the link's name and its points (as the IDE's designer).
  //
  // With a `mode` (DESIGN 5.31) the person changes what it shows, and each
  // gesture goes to the runtime as one action (`oncanvas`), which snaps it,
  // keeps it and chooses what it touched: in mode objects an object (an
  // `object` box and the shapes drawn after it) is dragged and dropped; in
  // mode paint the pen or eraser strokes the bound picture; in mode vector
  // the tools of its toolbar add shapes to the bound SVG drawing, select one
  // to drag it, Delete or Backspace delete it.
  import { anchorOf, cleanSvg, objectAt, objectsOf, shapeOfDrag, type CanvasObject } from "./canvasModes";
  import { colorOf, gridEvery, linkPoints, pointIn, scrolled, svgOf, wholeView, zoomed, type Shape, type View } from "../drawing";
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
    onbend = null,
    mode = null,
    picture = null,
    vector = null,
    tools = [],
    tool = null,
    brush = null,
    ink = null,
    toolWords = {},
    oncanvas = null,
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
    /** A link bent and let go: its name, and its bend points as whole numbers, x, y, x, y, .... */
    onbend?: ((bend: { name: string; via: number[] }) => void) | null;
    /** What the person does on the canvas; none: its draw handler draws it. */
    mode?: "objects" | "paint" | "vector" | null;
    /** Mode paint: the bound picture, as a URL; none for a blank one. */
    picture?: string | null;
    /** Mode vector: the bound drawing, as SVG text. */
    vector?: string | null;
    /** The tools of its mode the person may use, in the order its toolbar shows them. */
    tools?: string[];
    /** The tool, brush width and ink as the part has them (a handler sets them). */
    tool?: string | null;
    brush?: number | null;
    ink?: string | null;
    /** Clarity's words for the tools, and for the text the text tool asks, in the person's language. */
    toolWords?: Record<string, string>;
    /** An action of the person's on a canvas with a mode, for the runtime; false when it refused it. */
    oncanvas?: ((action: Record<string, unknown>) => Promise<boolean>) | null;
  } = $props();

  /** The tool in hand: the part's, else the one the person chose, else the first. */
  let chosenTool = $state<string | null>(null);
  $effect.pre(() => {
    // A handler that sets the tool sets the one in hand.
    if (tool) chosenTool = tool;
  });
  const toolNow = $derived(chosenTool && tools.includes(chosenTool) ? chosenTool : (tools[0] ?? null));
  const brushNow = $derived(brush ?? 2);

  /** A color as the drawing keeps it: written out, so other programs show it alike. */
  function inkNow(): string {
    const color = colorOf(ink ?? "text");
    const role = /^var\((--[\w-]+)/.exec(color);
    if (!role || !frame) return color.startsWith("#") ? color : "#000000";
    const value = getComputedStyle(frame).getPropertyValue(role[1]).trim();
    return value || "#000000";
  }

  /** Mode objects: the objects of the drawing, and the one being dragged with how far. */
  const objects = $derived(mode === "objects" ? objectsOf(drawing) : []);
  let objectHeld = $state<{ object: CanvasObject; id: number; start: { x: number; y: number }; dx: number; dy: number; dropped: boolean } | null>(null);
  /** Mode vector: the drawing's elements, the one chosen (from 0), and a move or a shape being made. */
  const vectorShown = $derived(mode === "vector" ? cleanSvg(vector ?? "", [units.width, units.height]) : null);
  let chosenShape = $state<number | null>(null);
  let shapeHeld = $state<{ index: number; id: number; start: { x: number; y: number }; dx: number; dy: number; dropped: boolean } | null>(null);
  let making = $state<{ id: number; points: { x: number; y: number }[] } | null>(null);
  /** The text tool's box, where the person clicked, until Enter or Escape. */
  let typing = $state<{ x: number; y: number; text: string } | null>(null);
  /** Mode paint: the stroke being painted, kept until the picture comes back with it. */
  let stroke = $state<{ id: number; points: { x: number; y: number }[]; done: boolean } | null>(null);
  // What comes back from the runtime shows the gesture: the canvas stops showing its own.
  $effect.pre(() => {
    void drawing;
    void vector;
    void picture;
    if (objectHeld?.dropped) objectHeld = null;
    if (shapeHeld?.dropped) shapeHeld = null;
    if (stroke?.done) stroke = null;
  });

  /** The canvas's own gesture in mode objects and vector: moved by how far, while it goes. */
  function heldMove(i: number): string | undefined {
    if (!objectHeld) return undefined;
    // By its record: a drawing drawn anew while it is dragged (a timer's tick) has it at another place.
    const held = objectHeld.object;
    const now = objects.find((o) => o.record === held.record) ?? held;
    if (i >= now.from && i < now.to) return `translate(${objectHeld.dx} ${objectHeld.dy})`;
    return undefined;
  }

  /** An element's attributes, moved while it is dragged. */
  function shifted(attrs: Record<string, string | number>, transform: string | undefined): Record<string, string | number> {
    return transform ? { ...attrs, transform } : attrs;
  }

  /** Sends an action, and lets the gesture go when the runtime refused it. */
  async function act(action: Record<string, unknown>) {
    const taken = await oncanvas?.(action);
    if (!taken) {
      objectHeld = null;
      shapeHeld = null;
      stroke = null;
    }
  }

  /** The press of a canvas with a mode; true when the mode took it. */
  function modeDown(e: PointerEvent): boolean {
    if (!mode || !oncanvas) return false;
    const p = unitAt(offset(e));
    if (mode === "objects") {
      const object = objectAt(objects, p);
      if (!object) return false;
      objectHeld = { object, id: e.pointerId, start: p, dx: 0, dy: 0, dropped: false };
    } else if (mode === "paint") {
      stroke = { id: e.pointerId, points: [p], done: false };
    } else if (toolNow === "select" || toolNow === null) {
      const index = shapeUnder(e);
      chosenShape = index;
      if (index === null) return false;
      shapeHeld = { index, id: e.pointerId, start: p, dx: 0, dy: 0, dropped: false };
    } else if (toolNow === "text") {
      typing = { x: Math.round(p.x), y: Math.round(p.y), text: "" };
      return true;
    } else {
      making = { id: e.pointerId, points: [p] };
    }
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    return true;
  }

  function modeMove(e: PointerEvent): boolean {
    const p = unitAt(offset(e));
    if (objectHeld && objectHeld.id === e.pointerId && !objectHeld.dropped) {
      objectHeld = { ...objectHeld, dx: p.x - objectHeld.start.x, dy: p.y - objectHeld.start.y };
      return true;
    }
    if (shapeHeld && shapeHeld.id === e.pointerId && !shapeHeld.dropped) {
      shapeHeld = { ...shapeHeld, dx: p.x - shapeHeld.start.x, dy: p.y - shapeHeld.start.y };
      return true;
    }
    if (making && making.id === e.pointerId) {
      making = { ...making, points: toolNow === "path" ? [...making.points, p] : [making.points[0], p] };
      return true;
    }
    if (stroke && stroke.id === e.pointerId && !stroke.done) {
      stroke = { ...stroke, points: [...stroke.points, p] };
      return true;
    }
    return false;
  }

  function modeUp(e: PointerEvent): boolean {
    if (objectHeld && objectHeld.id === e.pointerId && !objectHeld.dropped) {
      const held = objectHeld;
      if (!held.dx && !held.dy) {
        objectHeld = null;
        return true;
      }
      objectHeld = { ...held, dropped: true };
      void act({ move: { id: held.object.record, x: held.object.x + held.dx, y: held.object.y + held.dy } });
      return true;
    }
    if (shapeHeld && shapeHeld.id === e.pointerId && !shapeHeld.dropped) {
      const held = shapeHeld;
      const element = vectorShown?.shapes[held.index];
      const anchor = element ? anchorOf(element.tag, element.attrs) : null;
      if (!anchor || (!held.dx && !held.dy)) {
        shapeHeld = null;
        return true;
      }
      shapeHeld = { ...held, dropped: true };
      void act({ move: { index: held.index + 1, x: anchor.x + held.dx, y: anchor.y + held.dy } });
      return true;
    }
    if (making && making.id === e.pointerId) {
      const shape = toolNow ? shapeOfDrag(toolNow, making.points, inkNow(), brushNow) : null;
      making = null;
      if (shape) void act({ add: shape });
      return true;
    }
    if (stroke && stroke.id === e.pointerId && !stroke.done) {
      const points = stroke.points;
      stroke = { ...stroke, done: true };
      void painted(points).then((data) => (data ? act({ stroke: { data } }) : (stroke = null)));
      return true;
    }
    return false;
  }

  /** The element of the vector drawing under the pointer, from 0; null when none is. */
  function shapeUnder(e: PointerEvent): number | null {
    const hit = document.elementFromPoint?.(e.clientX, e.clientY);
    const place = hit?.closest?.("[data-shape]")?.getAttribute("data-shape");
    return place === null || place === undefined ? null : Number(place);
  }

  /** The picture after a stroke: the picture as it is with the stroke painted on, a PNG in base64. */
  async function painted(points: { x: number; y: number }[]): Promise<string | null> {
    const sheet = document.createElement("canvas");
    sheet.width = Math.max(1, Math.round(units.width));
    sheet.height = Math.max(1, Math.round(units.height));
    const pen = sheet.getContext("2d");
    if (!pen) return null;
    if (picture) {
      const image = new Image();
      image.src = picture;
      try {
        await image.decode();
        pen.drawImage(image, 0, 0, sheet.width, sheet.height);
      } catch {
        // A picture that does not load is painted over as blank.
      }
    }
    pen.lineWidth = brushNow;
    pen.lineCap = "round";
    pen.lineJoin = "round";
    pen.strokeStyle = inkNow();
    if (toolNow === "eraser") pen.globalCompositeOperation = "destination-out";
    pen.beginPath();
    points.forEach((p, i) => (i ? pen.lineTo(p.x, p.y) : pen.moveTo(p.x, p.y)));
    // A press that did not move paints a dot.
    if (points.length === 1) pen.lineTo(points[0].x + 0.01, points[0].y);
    pen.stroke();
    return sheet.toDataURL("image/png").split(",")[1] ?? null;
  }

  /** Delete or Backspace deletes the chosen shape of a vector drawing. */
  function key(e: KeyboardEvent) {
    if (mode !== "vector" || chosenShape === null || typing || !oncanvas) return;
    if (e.key !== "Delete" && e.key !== "Backspace") return;
    e.preventDefault();
    const index = chosenShape;
    chosenShape = null;
    void act({ delete: index + 1 });
  }

  /** The text tool's box: Enter adds the text there, Escape lets it go. */
  function typed(e: KeyboardEvent) {
    const box = typing;
    if (!box) return;
    if (e.key === "Escape") typing = null;
    if (e.key !== "Enter") return;
    e.preventDefault();
    const { x, y, text } = box;
    typing = null;
    if (text.trim()) void act({ add: { op: "text", x, y, text, color: inkNow(), size: Math.max(brushNow * 4, 10) } });
  }

  /** A link being bent: its points as the pointer has them, until the drawing comes back with them. */
  let bending = $state<{ name: string; via: { x: number; y: number }[] } | null>(null);
  /** The link last pressed, whose bend points show. */
  let inHand = $state<string | null>(null);
  // A drawing that comes back after a bend has the points: the canvas stops showing its own.
  $effect.pre(() => {
    void drawing;
    bending = null;
  });
  /** The drawing's shapes, a link being bent with the points the pointer gives it. */
  const shapesShown = $derived(
    drawing.map((shape) =>
      bending && shape.op === "link" && shape.name === bending.name ? { ...shape, via: bending.via.flatMap((p) => [p.x, p.y]) } : shape,
    ),
  );
  // A clear without a color is the canvas's background.
  const drawn = $derived(
    shapesShown.map((shape) => svgOf(shape.op === "clear" && !shape.color ? { ...shape, color: background || "panel" } : shape, units, shown, pictureUrl)),
  );
  /** The bend points of the link in hand, to drag or take away. */
  const handles = $derived.by(() => {
    if (!onbend || !inHand) return [];
    const link = shapesShown.find((shape) => shape.op === "link" && shape.name === inHand);
    return link ? linkPoints(link).slice(1, -1) : [];
  });

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

  /** A press on a link, bending it: which link, its bend points, the one in hand, and where it started. */
  let bendPress: { id: number; name: string; via: { x: number; y: number }[]; point: number; start: { x: number; y: number }; at: { x: number; y: number }; moved: boolean } | null = null;

  /** The unit under a pixel, not rounded. */
  function unitAt(at: { x: number; y: number }) {
    const v = viewNow();
    return { x: v[1] + at.x / v[0], y: v[2] + at.y / v[0] };
  }

  function distanceToSegment(p: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }) {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const length = dx * dx + dy * dy;
    const t = length === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / length));
    return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
  }

  const n = (value: unknown) => (typeof value === "number" ? value : Number(value) || 0);

  /** The link under a press, the topmost first: its bend point there, else a new one on the segment pressed. */
  function linkAt(p: { x: number; y: number }) {
    const near = 8 / viewNow()[0];
    // A shape drawn over the press, a box above a line, takes it: the link under it is not pressed.
    const covers = (shape: Shape) => shape.op === "rect" && p.x >= n(shape.x) && p.x <= n(shape.x) + n(shape.width) && p.y >= n(shape.y) && p.y <= n(shape.y) + n(shape.height);
    for (let i = shapesShown.length - 1; i >= 0; i--) {
      const shape = shapesShown[i];
      if (covers(shape)) return null;
      if (shape.op !== "link") continue;
      const all = linkPoints(shape);
      const via = all.slice(1, -1);
      const onPoint = via.findIndex((v) => Math.hypot(v.x - p.x, v.y - p.y) <= near);
      if (onPoint >= 0) return { name: String(shape.name ?? ""), via, point: onPoint };
      for (let i = 0; i < all.length - 1; i++) {
        if (distanceToSegment(p, all[i], all[i + 1]) <= near) return { name: String(shape.name ?? ""), via: [...via.slice(0, i), p, ...via.slice(i)], point: i };
      }
    }
    return null;
  }

  /** A double click on a bend point of the link in hand takes it away. */
  function unbend(e: MouseEvent) {
    if (!onbend || !inHand) return;
    const p = unitAt(offset(e));
    const near = 8 / viewNow()[0];
    const point = handles.findIndex((v) => Math.hypot(v.x - p.x, v.y - p.y) <= near);
    if (point < 0) return;
    const via = handles.filter((_, i) => i !== point);
    bending = { name: inHand, via };
    onbend({ name: inHand, via: flat(via) });
  }

  /** The bend points as a handler reads them: whole numbers, x, y, x, y, .... */
  const flat = (via: { x: number; y: number }[]) => via.flatMap((p) => [Math.round(p.x), Math.round(p.y)]);

  let frame = $state<HTMLDivElement | null>(null);

  function down(e: PointerEvent) {
    if (!e.isPrimary || e.button !== 0) return;
    frame?.focus({ preventScroll: true });
    if (modeDown(e)) return;
    if (onbend) {
      const at = offset(e);
      const found = linkAt(unitAt(at));
      if (found) {
        inHand = found.name;
        bendPress = { id: e.pointerId, ...found, start: found.via[found.point], at, moved: false };
        (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
        return;
      }
      inHand = null;
    }
    if (!(onpoint || ondrag || zoom)) return;
    const at = offset(e);
    const viewAt = viewNow();
    pressed = { id: e.pointerId, at, from: pointIn(units, viewAt, at), moved: false, viewAt, moves: ondrag ? "unknown" : "view", now: at };
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (modeMove(e)) return;
    if (bendPress && bendPress.id === e.pointerId) {
      const at = offset(e);
      if (!bendPress.moved && Math.hypot(at.x - bendPress.at.x, at.y - bendPress.at.y) < STILL) return;
      bendPress.moved = true;
      const scale = viewNow()[0];
      const via = [...bendPress.via];
      via[bendPress.point] = { x: bendPress.start.x + (at.x - bendPress.at.x) / scale, y: bendPress.start.y + (at.y - bendPress.at.y) / scale };
      bending = { name: bendPress.name, via };
      return;
    }
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
    if (modeUp(e)) return;
    if (bendPress && bendPress.id === e.pointerId) {
      const was = bendPress;
      bendPress = null;
      if (was.moved && bending) onbend?.({ name: was.name, via: flat(bending.via) });
      return;
    }
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
      // The part may have closed while the handler ran.
      if (was.moves === "view" || !svg) return;
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

<!-- It takes the focus when pressed, so Ctrl+Z takes back its own edits (DESIGN 5.29). -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions (Delete takes the chosen shape away) -->
<div class="canvas-frame" tabindex="-1" role="application" bind:this={frame} bind:clientWidth={partWidth} bind:clientHeight={partHeight} onkeydown={key}>
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
    onpointercancel={() => ((pressed = null), (bendPress = null), (objectHeld = null), (shapeHeld = null), (making = null), (stroke = null))}
    ondblclick={unbend}
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
    {#if mode === "paint" && picture}
      <image href={picture} x="0" y="0" width={units.width} height={units.height} preserveAspectRatio="none" />
    {/if}
    {#each elements as element, i (i)}
      <svelte:element this={element.tag} {...shifted(element.attrs, heldMove(i))}>{element.text ?? ""}</svelte:element>
      {#if dots && i === gridAfter}{@render gridShown()}{/if}
    {/each}
    {#if vectorShown}
      {#each vectorShown.shapes as element, i (i)}
        <svelte:element
          this={element.tag}
          {...shifted(element.attrs, shapeHeld?.index === i ? `translate(${shapeHeld.dx} ${shapeHeld.dy})` : undefined)}
          data-shape={i}
          class:chosen={chosenShape === i}>{element.text}</svelte:element>
      {/each}
    {/if}
    {#if making && toolNow}
      {@const shape = shapeOfDrag(toolNow, making.points, inkNow(), brushNow)}
      {#if shape}
        {@const shown = svgOf(shape, units, () => "")}
        <svelte:element this={shown.tag} {...shown.attrs} class="making" />
      {/if}
    {/if}
    {#if stroke}
      <polyline
        class="stroke"
        points={stroke.points.map((p) => `${p.x},${p.y}`).join(" ")}
        fill="none"
        stroke={toolNow === "eraser" ? "var(--panel)" : inkNow()}
        stroke-width={brushNow}
        stroke-linecap="round"
        stroke-linejoin="round" />
    {/if}
    {#each handles as point, p (p)}
      <circle class="bend" cx={point.x} cy={point.y} r={5 / view[0]} />
    {/each}
  </svg>
  {#if (mode === "paint" || mode === "vector") && tools.length > 1 && oncanvas}
    <!-- The mode's tools: the one in hand is pressed. -->
    <div class="tools" role="toolbar">
      {#each tools as t (t)}
        <button type="button" class:on={toolNow === t} aria-pressed={toolNow === t} title={toolWords[t] ?? t} onclick={() => (chosenTool = t)}>{toolWords[t] ?? t}</button>
      {/each}
    </div>
  {/if}
  {#if typing}
    {@const v = view}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="typing"
      style:left="{(typing.x - v[1]) * v[0]}px"
      style:top="{(typing.y - v[2]) * v[0]}px"
      placeholder={toolWords.canvasText ?? "The text"}
      aria-label={toolWords.canvasText ?? "The text"}
      autofocus
      bind:value={typing.text}
      onkeydown={typed}
      onblur={() => (typing = null)} />
  {/if}
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
  .canvas-frame:focus {
    outline: none;
  }

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

  /* The bend points of the link in hand: drag one to move it, double-click it to take it away. */
  .bend {
    fill: var(--panel);
    stroke: var(--accent);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
    cursor: move;
  }

  /* The chosen shape of a vector drawing. */
  .canvas-view :global(.chosen) {
    filter: drop-shadow(0 0 2px var(--accent));
  }

  .making,
  .stroke {
    pointer-events: none;
    opacity: 0.8;
  }

  .tools {
    position: absolute;
    top: 6px;
    left: 6px;
    display: flex;
    gap: 2px;
  }

  .tools button {
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--panel);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }

  .tools button.on {
    background: var(--accent);
    color: var(--on-accent, #fff);
  }

  .typing {
    position: absolute;
    min-width: 120px;
    font: inherit;
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
