// The drawing of the diagram designer's canvas, apart from the screen and
// from any kind of diagram: boxes with a title and rows of cells, where
// each stands, where lines meet them, and the marks at a line's ends. Each
// kind (ERD, class diagram) turns its model into shapes and links and
// takes the canvas's edits back (erdDesign.ts, classDesign.ts).

import type { Cardinality } from "./generated/Cardinality";

/** The height of a box's title, of its stereotype line, of a row, and the room below the last row. */
export const TITLE = 32;
export const STEREOTYPE = 14;
export const ROW = 22;
export const FOOT = 6;
/** The room around the text in a box, and between the cells of a row. */
const PAD = 12;
const GAP = 14;
/** How boxes without a place are laid out: at most this many in a row, this far apart. */
const GRID_GAP = 60;

export type Point = { x: number; y: number };
export type Rect = { x: number; y: number; w: number; h: number };

/** What a kind gives the canvas for a box: its name, place, rows of cells, and how its title looks. */
export type Shape = {
  name: string;
  x: number | null;
  y: number | null;
  rows: string[][];
  /** Shown small above the name, as «enumeration». */
  stereotype?: string | null;
  /** The title's color: entity, enum, engine or module. */
  tone?: string;
  /** How each cell column looks: type, field or keys. */
  cells?: string[];
  /** The group it is drawn in a frame with: a module, as a namespace. */
  group?: string | null;
};

/** A shape as drawn: where it stands and how big it is. */
export type Box = Rect & { name: string; rows: string[][]; stereotype: string | null; tone: string; placed: boolean; title: number; cells?: string[]; group?: string | null };

/** One end of a line: an ERD's count (bars, ring, crow's foot), or a UML head with its multiplicity. */
export type EndStyle =
  | { kind: "count"; count: Cardinality }
  | { kind: "uml"; head: "none" | "arrow" | "diamond" | "filledDiamond"; text: string };

/** A line between two boxes, as the canvas draws it. */
export type Link = {
  left: string;
  right: string;
  leftEnd: EndStyle;
  rightEnd: EndStyle;
  dashed: boolean;
  label: string | null;
  /** The points the line bends at, from left to right; none: straight. */
  via?: Point[];
};

/** What is picked on the canvas: a box by name, or a line by its place in the list. */
export type CanvasPick = { kind: "box"; name: string } | { kind: "link"; index: number } | null;

/** What the person did on the canvas, for the kind to make in its model. */
export type CanvasEdit =
  | { op: "add"; item: string; x: number; y: number }
  | { op: "move"; name: string; x: number; y: number }
  | { op: "moveAll"; moves: { name: string; x: number; y: number }[] }
  | { op: "connect"; tool: string; from: string; to: string }
  /** A line's bend points, all of them as they are now. */
  | { op: "bend"; index: number; via: Point[] };

/** An item of the palette: a box to drag onto the canvas, or a tool that draws lines. */
export type PaletteItem = { id: string; label: string; glyph: string; tool: boolean; title: string };

/** The width of each cell column of `rows`. */
function cellWidths(rows: string[][], measure: (text: string) => number): number[] {
  const count = Math.max(0, ...rows.map((r) => r.length));
  return Array.from({ length: count }, (_, i) => Math.max(0, ...rows.map((r) => measure(r[i] ?? ""))));
}

/** Each shape's box: its size from its text, its place as given or the next free place of the layout. */
/** A line as the layout reads it: its two ends, and what is drawn at each, which says which is the whole. */
export type LaidLine = { left: string; right: string; leftEnd?: EndStyle; rightEnd?: EndStyle };

export function boxes(shapes: Shape[], measure: (text: string) => number, lines: LaidLine[] = []): Box[] {
  const sized = shapes.map((shape) => {
    const widths = cellWidths(shape.rows, measure);
    const used = widths.filter((w) => w > 0);
    const rowWidth = used.reduce((a, b) => a + b, 0) + GAP * Math.max(0, used.length - 1);
    const stereotype = shape.stereotype ?? null;
    const titleText = Math.max(measure(shape.name), stereotype ? measure(`«${stereotype}»`) : 0);
    const w = Math.ceil(Math.max(titleText + 2 * PAD, rowWidth + 2 * PAD, 120));
    const title = TITLE + (stereotype ? STEREOTYPE : 0);
    const h = title + ROW * shape.rows.length + FOOT;
    return { shape, w, h, title, stereotype };
  });
  const places = layout(sized.map(({ shape, w, h }) => ({ shape, w, h })), lines);
  return sized.map(({ shape, w, h, title, stereotype }) => {
    const base = { name: shape.name, rows: shape.rows, stereotype, tone: shape.tone ?? "entity", cells: shape.cells ?? [], group: shape.group ?? null, title, w, h };
    if (shape.x !== null && shape.y !== null) return { ...base, x: shape.x, y: shape.y, placed: true };
    const at = places.get(shape.name) ?? { x: 40, y: 40 };
    return { ...base, ...at, placed: false };
  });
}

/**
 * Places for the shapes without one, beside the placed ones: a group at a
 * time in the order the groups are first met (boxes without a group last),
 * each group laid out in layers (`layered`) and starting far enough below
 * the one above for both frames.
 */
function layout(items: { shape: Shape; w: number; h: number }[], lines: LaidLine[]): Map<string, Point> {
  const placed = items.filter(({ shape }) => shape.x !== null && shape.y !== null);
  const startX = placed.length > 0 ? Math.max(...placed.map(({ shape, w }) => (shape.x ?? 0) + w)) + GRID_GAP : 40;
  const open = items.filter(({ shape }) => shape.x === null || shape.y === null);
  const order = [...new Set(open.map(({ shape }) => shape.group ?? null))].sort((a, b) => (a === null ? 1 : 0) - (b === null ? 1 : 0));
  const places = new Map<string, Point>();
  let y = 40;
  for (const group of order) {
    const members = open.filter(({ shape }) => (shape.group ?? null) === group);
    const laid = layered(members.map(({ shape, w, h }) => ({ name: shape.name, w, h })), lines);
    let bottom = y;
    for (const [name, at] of laid) {
      const member = members.find(({ shape }) => shape.name === name)!;
      places.set(name, { x: startX + at.x, y: y + at.y });
      bottom = Math.max(bottom, y + at.y + member.h);
    }
    y = bottom + GRID_GAP + 2 * FRAME_PAD + FRAME_LABEL;
  }
  return places;
}

/** The space between layers, and between boxes in a layer. */
const LAYER_GAP = 90;

/** Whether an end is the whole of what the line owns or lists: a diamond, as UML draws it. */
const isWhole = (end?: EndStyle) => end?.kind === "uml" && (end.head === "filledDiamond" || end.head === "diamond");

/**
 * Places for boxes in layers, from (0, 0): what is referred to stands above
 * what refers to it, and a whole above its parts. Lines that go round in a
 * circle are cut where they close it. Each layer is ordered by where the
 * boxes' neighbours are, swept down and up a few times, so lines cross
 * less; each is a row, centred on the widest. Boxes no line reaches stand
 * in a row of their own, below.
 */
export function layered(items: { name: string; w: number; h: number }[], lines: LaidLine[]): Map<string, Point> {
  const names = new Set(items.map((i) => i.name));
  // Each line once, from the box above to the box below.
  const edges: [string, string][] = [];
  for (const l of lines) {
    if (l.left === l.right || !names.has(l.left) || !names.has(l.right)) continue;
    edges.push(isWhole(l.leftEnd) ? [l.left, l.right] : [l.right, l.left]);
  }
  const below = new Map<string, string[]>(items.map((i) => [i.name, []]));
  // Lines that close a circle are left out, found by walking down from each box in the order given.
  const state = new Map<string, "walking" | "done">();
  const kept: [string, string][] = [];
  const walk = (name: string) => {
    state.set(name, "walking");
    for (const [up, down] of edges) {
      if (up !== name) continue;
      if (state.get(down) === "walking") continue;
      kept.push([up, down]);
      if (!state.has(down)) walk(down);
    }
    state.set(name, "done");
  };
  for (const { name } of items) if (!state.has(name)) walk(name);
  for (const [up, down] of kept) below.get(up)!.push(down);
  // A box's layer is the longest way down to it from a box nothing is above.
  const layer = new Map<string, number>();
  const above = new Map<string, string[]>(items.map((i) => [i.name, []]));
  for (const [up, down] of kept) above.get(down)!.push(up);
  const layerOf = (name: string): number => {
    if (layer.has(name)) return layer.get(name)!;
    layer.set(name, 0);
    const at = Math.max(-1, ...above.get(name)!.map(layerOf)) + 1;
    layer.set(name, at);
    return at;
  };
  const connected = new Set(kept.flat());
  const linked = items.filter((i) => connected.has(i.name));
  const loose = items.filter((i) => !connected.has(i.name));
  for (const { name } of linked) layerOf(name);
  const count = Math.max(0, ...linked.map((i) => layer.get(i.name)!)) + 1;
  const rows: string[][] = Array.from({ length: count }, () => []);
  for (const { name } of linked) rows[layer.get(name)!].push(name);
  // Order each layer by the mean place of its neighbours in the layer above, then below.
  const neighbours = (name: string) => [...above.get(name)!, ...below.get(name)!];
  const placeOf = () => new Map(rows.flatMap((row) => row.map((name, i) => [name, i / Math.max(1, row.length - 1)] as const)));
  const sweep = (from: number, to: number, step: number) => {
    for (let r = from; r !== to; r += step) {
      const at = placeOf();
      const mean = (name: string) => {
        const near = neighbours(name).filter((n) => layer.get(n) === r - step);
        return near.length ? near.reduce((sum, n) => sum + at.get(n)!, 0) / near.length : at.get(name)!;
      };
      rows[r].sort((a, b) => mean(a) - mean(b));
    }
  };
  for (let pass = 0; pass < 4; pass++) {
    sweep(1, count, 1);
    sweep(count - 2, -1, -1);
  }
  if (loose.length > 0) rows.push(loose.map((i) => i.name));
  // Each row centred on the widest, the rows one below the other.
  const size = new Map(items.map((i) => [i.name, i]));
  const width = (row: string[]) => row.reduce((sum, n) => sum + size.get(n)!.w, 0) + LAYER_GAP * Math.max(0, row.length - 1);
  const widest = Math.max(0, ...rows.map(width));
  const places = new Map<string, Point>();
  let y = 0;
  for (const row of rows) {
    let x = (widest - width(row)) / 2;
    for (const name of row) {
      places.set(name, { x: Math.round(x), y });
      x += size.get(name)!.w + LAYER_GAP;
    }
    y += Math.max(0, ...row.map((n) => size.get(n)!.h)) + LAYER_GAP;
  }
  return places;
}

export const FRAME_PAD = 16;
export const FRAME_LABEL = 20;

/** A group's frame: around its boxes, with room above them for its name. */
export type Frame = Rect & { name: string };

/** A frame around the boxes of each group, in the order the groups are first met. */
export function frames(shown: Box[]): Frame[] {
  const groups = new Map<string, Box[]>();
  for (const b of shown) {
    if (b.group) groups.set(b.group, [...(groups.get(b.group) ?? []), b]);
  }
  return [...groups].map(([name, members]) => {
    const left = Math.min(...members.map((b) => b.x));
    const top = Math.min(...members.map((b) => b.y));
    const right = Math.max(...members.map((b) => b.x + b.w));
    const bottom = Math.max(...members.map((b) => b.y + b.h));
    return { name, x: left - FRAME_PAD, y: top - FRAME_PAD - FRAME_LABEL, w: right - left + 2 * FRAME_PAD, h: bottom - top + 2 * FRAME_PAD + FRAME_LABEL };
  });
}

/** Where each cell column of a box's rows starts. */
export function columns(box: Box, measure: (text: string) => number): number[] {
  const widths = cellWidths(box.rows, measure);
  const starts: number[] = [];
  let x = box.x + PAD;
  for (const w of widths) {
    starts.push(x);
    if (w > 0) x += w + GAP;
  }
  return starts;
}

function center(r: Rect): Point {
  return { x: r.x + r.w / 2, y: r.y + r.h / 2 };
}

/** Where the line from the center of `r` toward `toward` leaves `r`. */
function edge(r: Rect, toward: Point): Point {
  const c = center(r);
  const dx = toward.x - c.x;
  const dy = toward.y - c.y;
  if (dx === 0 && dy === 0) return c;
  const scale = Math.min(dx === 0 ? Infinity : r.w / 2 / Math.abs(dx), dy === 0 ? Infinity : r.h / 2 / Math.abs(dy));
  return { x: c.x + dx * scale, y: c.y + dy * scale };
}

/** Where a line between `a` and `b` meets each of them, on the line between their centers. */
export function meeting(a: Rect, b: Rect): [Point, Point] {
  return [edge(a, center(b)), edge(b, center(a))];
}

type Side = "left" | "right" | "top" | "bottom";

/** The side of `r` the line from its center toward `toward` leaves by. */
function sideOf(r: Rect, toward: Point): Side {
  const c = center(r);
  const dx = toward.x - c.x;
  const dy = toward.y - c.y;
  if (Math.abs(dx) * r.h >= Math.abs(dy) * r.w) return dx >= 0 ? "right" : "left";
  return dy >= 0 ? "bottom" : "top";
}

/**
 * Where each line meets its two boxes, [left end, right end], or null for
 * a line to itself or to a box not shown. Ends on the same side of a box
 * are spread along it, in the order of the boxes at their other ends, so
 * lines that meet one box do not lie on each other.
 */
export function ends(shown: Box[], lines: { left: string; right: string; via?: Point[] }[]): ([Point, Point] | null)[] {
  const byName = new Map(shown.map((b) => [b.name, b]));
  // Each end faces what it goes to: the line's nearest bend point, else the other box.
  type End = { line: number; at: 0 | 1; box: Box; aim: Point; side: Side };
  const all: End[] = [];
  lines.forEach((l, line) => {
    const left = byName.get(l.left);
    const right = byName.get(l.right);
    if (!left || !right || left === right) return;
    const first = l.via?.[0] ?? center(right);
    const last = l.via?.at(-1) ?? center(left);
    all.push({ line, at: 0, box: left, aim: first, side: sideOf(left, first) });
    all.push({ line, at: 1, box: right, aim: last, side: sideOf(right, last) });
  });
  const result: ([Point, Point] | null)[] = lines.map(() => null);
  const points = new Map<End, Point>();
  const groups = new Map<Box, Map<Side, End[]>>();
  for (const end of all) {
    const sides = groups.get(end.box) ?? new Map<Side, End[]>();
    sides.set(end.side, [...(sides.get(end.side) ?? []), end]);
    groups.set(end.box, sides);
  }
  for (const group of [...groups.values()].flatMap((sides) => [...sides.values()])) {
    const across = (e: End) => (e.side === "left" || e.side === "right" ? e.aim.y : e.aim.x);
    group.sort((a, b) => across(a) - across(b) || a.line - b.line);
    group.forEach((end, i) => {
      const share = (i + 1) / (group.length + 1);
      const { x, y, w, h } = end.box;
      const point =
        end.side === "left" ? { x, y: y + h * share }
        : end.side === "right" ? { x: x + w, y: y + h * share }
        : end.side === "top" ? { x: x + w * share, y }
        : { x: x + w * share, y: y + h };
      points.set(end, point);
    });
  }
  for (const end of all) {
    const pair = (result[end.line] ??= [center(end.box), center(end.box)]);
    pair[end.at] = points.get(end)!;
  }
  return result;
}

function pointAt(at: Point, along: Point) {
  const across = { x: -along.y, y: along.x };
  return (d: number, side: number) => ({ x: at.x + along.x * d + across.x * side, y: at.y + along.y * d + across.y * side });
}

const segment = (a: Point, b: Point) => `M${a.x.toFixed(1)},${a.y.toFixed(1)}L${b.x.toFixed(1)},${b.y.toFixed(1)}`;

/**
 * The marks at one end of an ERD line, at `at` on the box's edge, with
 * `along` the unit direction from the box out along the line: bars for
 * one, a ring for zero, a crow's foot for many. `path` draws the bars and
 * the foot.
 */
export function endMark(at: Point, along: Point, count: Cardinality): { path: string; ring: boolean; ringAt: Point; bars: number; foot: boolean } {
  const p = pointAt(at, along);
  const foot = count === "zeroOrMore" || count === "oneOrMore";
  const ring = count === "zeroOrOne" || count === "zeroOrMore";
  const bars = count === "exactlyOne" ? 2 : count === "oneOrMore" || count === "zeroOrOne" ? 1 : 0;
  const parts: string[] = [];
  if (foot) parts.push(segment(p(0, -7), p(12, 0)), segment(p(0, 0), p(12, 0)), segment(p(0, 7), p(12, 0)));
  for (let i = 0; i < bars; i++) parts.push(segment(p(foot ? 16 : 8 + i * 5, -7), p(foot ? 16 : 8 + i * 5, 7)));
  return { path: parts.join(""), ring, ringAt: p(foot ? 22 : 18, 0), bars, foot };
}

/**
 * The head at one end of a UML line: an open arrow, or a diamond (filled
 * for a composition) whose point touches the box. `textAt` is where the
 * multiplicity is written, beside the line.
 */
export function umlMark(at: Point, along: Point, head: "none" | "arrow" | "diamond" | "filledDiamond"): { path: string; closed: boolean; filled: boolean; textAt: Point } {
  const p = pointAt(at, along);
  const textAt = p(head === "diamond" || head === "filledDiamond" ? 26 : 14, 10);
  const dot = (q: Point) => `${q.x.toFixed(1)},${q.y.toFixed(1)}`;
  switch (head) {
    case "arrow":
      return { path: segment(p(11, -6), at) + segment(at, p(11, 6)), closed: false, filled: false, textAt };
    case "diamond":
    case "filledDiamond":
      return { path: `M${dot(at)}L${dot(p(9, -5))}L${dot(p(18, 0))}L${dot(p(9, 5))}Z`, closed: true, filled: head === "filledDiamond", textAt };
    default:
      return { path: "", closed: false, filled: false, textAt };
  }
}

/** Gives every item without a place the place its box is shown at, so an edit moves nothing else. */
export function pinned<T extends { name: string; x: number | null; y: number | null }>(items: T[], shown: Box[]): T[] {
  if (items.every((e) => e.x !== null && e.y !== null)) return items;
  return items.map((e) => {
    if (e.x !== null && e.y !== null) return e;
    const box = shown.find((b) => b.name === e.name);
    return box ? { ...e, x: Math.round(box.x), y: Math.round(box.y) } : e;
  });
}

/** The first free name of `base` followed by a number: Entity1, Entity2... */
export function freeName(base: string, taken: string[]): string {
  let n = 1;
  while (taken.includes(`${base}${n}`)) n++;
  return `${base}${n}`;
}

/** Bend points as a diagram keeps them: whole units, [x, y]. */
export function wholePoints(via: Point[]): [number, number][] {
  return via.map((p) => [Math.round(p.x), Math.round(p.y)]);
}
