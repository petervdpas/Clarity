// Reordering (DESIGN 5.30), as every renderer does it (conformance/reorder.json):
// a row of a list, table, tree or tree table moved by a drag or by Alt and
// an arrow key. The result is the list's new order, each row's parent (in a
// tree) and each row's number at its level, which the part writes into the
// field of `reorder by`. A move that changes no parent or number is refused.

import { parentsOf, type TreeRow } from "./trees.ts";

/** A drag onto a row, or a key on the moved row; places count from 0 in the list. */
export type Move =
  | { row: number; where: "before" | "after" | "into"; target: number }
  | { row: number; key: "up" | "down" | "left" | "right" };

/** The list after a move: the old places in the new order, each row's parent and number in that order. */
export type Reordered = { order: number[]; parents: unknown[]; numbers: number[] };

/** Each row's place at its level counted from 1, in the order of the list. */
function numbered(levels: number[], order: number[]): number[] {
  const counts = new Map<number, number>();
  return order.map((place) => {
    const n = (counts.get(levels[place]) ?? 0) + 1;
    counts.set(levels[place], n);
    return n;
  });
}

/** The rows after `move`, or null when it changes nothing. In a list that is no tree, rows have no parent. */
export function reorder(rows: { id: unknown; parent?: unknown }[], move: Move, tree: boolean): Reordered | null {
  const r = move.row;
  if (r < 0 || r >= rows.length) return null;
  const asTree: TreeRow[] = rows.map((row) => ({ id: row.id, parent: tree ? (row.parent ?? null) : null }));
  const levels = parentsOf(asTree);
  const level = (place: number) => levels.map((l, i) => (l === place ? i : -1)).filter((i) => i >= 0);
  const under = (place: number, row: number) => {
    for (let p = place; p !== -1; p = levels[p]) if (p === row) return true;
    return false;
  };
  // The last of a row and the rows right under it, leaving out the moved row.
  const lastOf = (place: number) => Math.max(place, ...level(place).filter((i) => i !== r));
  let parent = asTree[r].parent;
  let anchor: number;
  let after: boolean;
  if ("key" in move) {
    const siblings = level(levels[r]);
    const k = siblings.indexOf(r);
    if (move.key === "up" || move.key === "down") {
      const next = move.key === "up" ? k - 1 : k + 1;
      if (next < 0 || next >= siblings.length) return null;
      [anchor, after] = [siblings[next], move.key === "down"];
    } else if (!tree) {
      return null;
    } else if (move.key === "left") {
      if (levels[r] === -1) return null;
      [anchor, after, parent] = [levels[r], true, asTree[levels[r]].parent];
    } else {
      if (k === 0) return null;
      const before = siblings[k - 1];
      [anchor, after, parent] = [lastOf(before), true, asTree[before].id];
    }
  } else {
    const t = move.target;
    if (t < 0 || t >= rows.length || t === r || (tree && under(t, r))) return null;
    if (move.where === "into") {
      if (!tree) return null;
      [anchor, after, parent] = [lastOf(t), true, asTree[t].id];
    } else {
      [anchor, after, parent] = [t, move.where === "after", tree ? asTree[t].parent : null];
    }
  }
  const order = asTree.map((_, i) => i).filter((i) => i !== r);
  order.splice(order.indexOf(anchor) + (after ? 1 : 0), 0, r);
  const moved = asTree.map((row, i) => (i === r ? { ...row, parent } : row));
  const newLevels = parentsOf(order.map((i) => moved[i]));
  // Levels by old place, so both lists are numbered alike.
  const byPlace = new Array<number>(rows.length);
  order.forEach((place, i) => (byPlace[place] = newLevels[i] === -1 ? -1 : order[newLevels[i]]));
  const numbers = numbered(byPlace, order);
  const before = numbered(levels, asTree.map((_, i) => i));
  const same = order.every((place, i) => numbers[i] === before[place] && moved[place].parent === asTree[place].parent);
  if (same) return null;
  return { order, parents: order.map((i) => moved[i].parent), numbers };
}
