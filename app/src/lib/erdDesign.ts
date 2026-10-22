// The ERD designer's drawing and edits, apart from the screen: how big a
// box is and where it stands, where a relation meets a box, the marks that
// say how many at each end, and the edits the canvas makes. The backend
// reads and writes the Mermaid text (read_erd, write_erd).

import type { Cardinality } from "./generated/Cardinality";
import type { Erd } from "./generated/Erd";
import type { ErdEntity } from "./generated/ErdEntity";

/** The height of a box's title, of a field's row, and the room below the last row. */
export const TITLE = 32;
export const ROW = 22;
export const FOOT = 6;
/** The room around the text in a box, and between the columns of a row. */
const PAD = 12;
const GAP = 14;
/** The grid boxes without a place go into, beside the placed ones. */
const GRID_X = 260;
const GRID_Y = 200;
const GRID_COLUMNS = 4;

export type Point = { x: number; y: number };
export type Rect = { x: number; y: number; w: number; h: number };
export type Row = { type: string; name: string; keys: string };
export type Box = Rect & { name: string; rows: Row[]; placed: boolean };

/** A field's row: its type, its name, and its keys (FK for a ref, UK for unique, * for required). */
export function rowsOf(entity: ErdEntity): Row[] {
  return entity.fields.map((f) => {
    const keys = [f.typeText.startsWith("ref ") ? "FK" : "", f.unique ? "UK" : "", f.required ? "*" : ""].filter(Boolean).join(" ");
    return { type: f.typeText, name: f.name, keys };
  });
}

/** Each entity's box: its size from its text, its place as written or the next free cell of the grid. */
export function boxes(erd: Erd, measure: (text: string) => number): Box[] {
  const placed = erd.entities.filter((e) => e.x !== null && e.y !== null);
  const startX = placed.length > 0 ? Math.max(...placed.map((e) => e.x ?? 0)) + GRID_X : 40;
  let next = 0;
  return erd.entities.map((entity) => {
    const rows = rowsOf(entity);
    const typeWidth = Math.max(0, ...rows.map((r) => measure(r.type)));
    const nameWidth = Math.max(0, ...rows.map((r) => measure(r.name)));
    const keyWidth = Math.max(0, ...rows.map((r) => measure(r.keys)));
    const w = Math.ceil(Math.max(measure(entity.name) + 2 * PAD, typeWidth + nameWidth + keyWidth + 2 * GAP + 2 * PAD, 120));
    const h = TITLE + ROW * rows.length + FOOT;
    if (entity.x !== null && entity.y !== null) return { name: entity.name, rows, x: entity.x, y: entity.y, w, h, placed: true };
    const cell = next++;
    return { name: entity.name, rows, x: startX + (cell % GRID_COLUMNS) * GRID_X, y: 40 + Math.floor(cell / GRID_COLUMNS) * GRID_Y, w, h, placed: false };
  });
}

/** Where the column of a row starts: the type, the name and the keys. */
export function columns(box: Box, measure: (text: string) => number): { type: number; name: number; keys: number } {
  const typeWidth = Math.max(0, ...box.rows.map((r) => measure(r.type)));
  const nameWidth = Math.max(0, ...box.rows.map((r) => measure(r.name)));
  return { type: box.x + PAD, name: box.x + PAD + typeWidth + GAP, keys: box.x + PAD + typeWidth + GAP + nameWidth + GAP };
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

/** Where a relation between `a` and `b` meets each of them. */
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
 * Where each relation meets its two boxes, [left end, right end], or null
 * for a relation to itself or to a box not shown. Ends on the same side of
 * a box are spread along it, in the order of the boxes at their other ends,
 * so lines that meet one box do not lie on each other.
 */
export function ends(shown: Box[], relations: Erd["relations"]): ([Point, Point] | null)[] {
  const byName = new Map(shown.map((b) => [b.name, b]));
  type End = { relation: number; at: 0 | 1; box: Box; other: Box; side: Side };
  const all: End[] = [];
  relations.forEach((r, relation) => {
    const left = byName.get(r.left);
    const right = byName.get(r.right);
    if (!left || !right || left === right) return;
    all.push({ relation, at: 0, box: left, other: right, side: sideOf(left, center(right)) });
    all.push({ relation, at: 1, box: right, other: left, side: sideOf(right, center(left)) });
  });
  const result: ([Point, Point] | null)[] = relations.map(() => null);
  const points = new Map<End, Point>();
  const groups = new Map<string, End[]>();
  for (const end of all) {
    const key = `${end.box.name}\u0000${end.side}`;
    groups.set(key, [...(groups.get(key) ?? []), end]);
  }
  for (const group of groups.values()) {
    const across = (e: End) => (e.side === "left" || e.side === "right" ? center(e.other).y : center(e.other).x);
    group.sort((a, b) => across(a) - across(b) || a.relation - b.relation);
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
    const pair = (result[end.relation] ??= [center(end.box), center(end.other)]);
    pair[end.at] = points.get(end)!;
  }
  return result;
}

/**
 * The marks at one end of a relation, at `at` on the box's edge, with `along`
 * the unit direction from the box out along the line: bars for one, a ring
 * for zero, a crow's foot for many. `path` draws the bars and the foot.
 */
export function endMark(at: Point, along: Point, count: Cardinality): { path: string; ring: boolean; ringAt: Point; bars: number; foot: boolean } {
  const across = { x: -along.y, y: along.x };
  const p = (d: number, side: number) => ({ x: at.x + along.x * d + across.x * side, y: at.y + along.y * d + across.y * side });
  const segment = (a: Point, b: Point) => `M${a.x.toFixed(1)},${a.y.toFixed(1)}L${b.x.toFixed(1)},${b.y.toFixed(1)}`;
  const foot = count === "zeroOrMore" || count === "oneOrMore";
  const ring = count === "zeroOrOne" || count === "zeroOrMore";
  const bars = count === "exactlyOne" ? 2 : count === "oneOrMore" || count === "zeroOrOne" ? 1 : 0;
  const parts: string[] = [];
  if (foot) parts.push(segment(p(0, -7), p(12, 0)), segment(p(0, 0), p(12, 0)), segment(p(0, 7), p(12, 0)));
  for (let i = 0; i < bars; i++) parts.push(segment(p(foot ? 16 : 8 + i * 5, -7), p(foot ? 16 : 8 + i * 5, 7)));
  return { path: parts.join(""), ring, ringAt: p(foot ? 22 : 18, 0), bars, foot };
}

function rename(erd: Erd, from: string, to: string): Erd {
  return {
    entities: erd.entities.map((e) => (e.name === from ? { ...e, name: to } : e)),
    relations: erd.relations.map((r) => ({ ...r, left: r.left === from ? to : r.left, right: r.right === from ? to : r.right })),
  };
}

/** Adds an entity at (x, y), named Entity1, Entity2... as the first free name. */
export function addEntity(erd: Erd, x: number, y: number): Erd {
  let n = 1;
  while (erd.entities.some((e) => e.name === `Entity${n}`)) n++;
  return { ...erd, entities: [...erd.entities, { name: `Entity${n}`, x: Math.round(x), y: Math.round(y), fields: [] }] };
}

/** Puts an entity's box at (x, y). */
export function moveEntity(erd: Erd, name: string, x: number, y: number): Erd {
  return { ...erd, entities: erd.entities.map((e) => (e.name === name ? { ...e, x: Math.round(x), y: Math.round(y) } : e)) };
}

/** Renames an entity and its relations; the same ERD when the name is empty or in use. */
export function renameEntity(erd: Erd, from: string, to: string): Erd {
  const name = to.trim();
  if (!name || name === from || erd.entities.some((e) => e.name === name)) return erd;
  return rename(erd, from, name);
}

/** Takes an entity away, with its relations. */
export function removeEntity(erd: Erd, name: string): Erd {
  return { entities: erd.entities.filter((e) => e.name !== name), relations: erd.relations.filter((r) => r.left !== name && r.right !== name) };
}

/** Draws a relation from `from` to `to`: many of `from` refer to one of `to`. */
export function connect(erd: Erd, from: string, to: string): Erd {
  if (from === to && erd.relations.some((r) => r.left === from && r.right === to)) return erd;
  if (erd.relations.some((r) => (r.left === from && r.right === to) || (r.left === to && r.right === from))) return erd;
  return { ...erd, relations: [...erd.relations, { left: from, right: to, leftCount: "zeroOrMore", rightCount: "exactlyOne", identifying: false, label: null }] };
}

/** Gives every entity without a place the place its box is shown at, so an edit moves nothing else. */
export function pinned(erd: Erd, shown: Box[]): Erd {
  if (erd.entities.every((e) => e.x !== null && e.y !== null)) return erd;
  return {
    ...erd,
    entities: erd.entities.map((e) => {
      if (e.x !== null && e.y !== null) return e;
      const box = shown.find((b) => b.name === e.name);
      return box ? { ...e, x: Math.round(box.x), y: Math.round(box.y) } : e;
    }),
  };
}
