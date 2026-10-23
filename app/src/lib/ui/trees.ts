// Trees of U1c, as every renderer shows them (conformance/trees.json): the
// rows of a list arranged by their parent, a row under its parent when that
// is in the list and at the top otherwise, each level in the order of the
// list. A loop of parents is broken at its row that comes first in the list.

/** A row as a tree places it: its id, and the id of its parent or none. */
export type TreeRow = { id: unknown; parent: unknown };

/** A row shown: its place in the list, its depth, and whether rows are under it. */
export type Shown = { place: number; depth: number; children: boolean };

/** The place of each row's parent in the list, or -1 for a row at the top. */
export function parentsOf(rows: TreeRow[]): number[] {
  const at = new Map<unknown, number>();
  rows.forEach((r, i) => {
    if (!at.has(r.id)) at.set(r.id, i);
  });
  const parent = rows.map((r) => (r.parent === null || r.parent === undefined ? -1 : (at.get(r.parent) ?? -1)));
  // Break each loop at its row that comes first in the list.
  rows.forEach((_, i) => {
    const seen: number[] = [];
    let p = i;
    while (p !== -1 && !seen.includes(p)) {
      seen.push(p);
      p = parent[p];
    }
    if (p !== -1) {
      const loop = seen.slice(seen.indexOf(p));
      parent[Math.min(...loop)] = -1;
    }
  });
  return parent;
}

/** The rows shown, top to bottom, with `open` the ids of the rows shown open. */
export function shownTree(rows: TreeRow[], open: Set<unknown>): Shown[] {
  const parent = parentsOf(rows);
  const under = rows.map(() => [] as number[]);
  const tops: number[] = [];
  parent.forEach((p, i) => (p === -1 ? tops : under[p]).push(i));
  const out: Shown[] = [];
  const walk = (place: number, depth: number) => {
    out.push({ place, depth, children: under[place].length > 0 });
    if (open.has(rows[place].id)) for (const child of under[place]) walk(child, depth + 1);
  };
  for (const top of tops) walk(top, 0);
  return out;
}

/** The ids of the rows above the row at `at`, top first: what a selected row opens. */
export function above(rows: TreeRow[], at: number): unknown[] {
  const parent = parentsOf(rows);
  const ids: unknown[] = [];
  for (let p = parent[at]; p !== -1 && p !== undefined; p = parent[p]) ids.unshift(rows[p].id);
  return ids;
}
