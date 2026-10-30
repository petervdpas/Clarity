// Reordering through the real Table part (DESIGN 5.30): Alt and an arrow
// moves the chosen row, a drag drops a row before, after or (in a tree)
// into another, and in the designer nothing moves.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import Table, { type Row } from "./Table.svelte";
import type { Move } from "./reorder";

const columns = [{ field: "title", label: "Title", kind: "text" as const, edit: false }];

function table(props: Record<string, unknown>) {
  const target = document.createElement("div");
  document.body.append(target);
  const moves: Move[] = [];
  const part = mount(Table, {
    target,
    props: { columns, rows: [] as Row[], onselect: () => {}, onopen: () => {}, onedit: () => {}, onreorder: (m: Move) => moves.push(m), ...props },
  });
  flushSync();
  return { target, moves, done: () => (unmount(part), target.remove()) };
}

const tasks: Row[] = [
  { id: 1, title: "One", parent: null },
  { id: 2, title: "Two", parent: null },
  { id: 3, title: "Three", parent: null },
];

/** A row's box: rows of 20 pixels, the first at the top. */
function place(target: HTMLElement) {
  target.querySelectorAll<HTMLElement>("tbody tr").forEach((tr, i) => {
    tr.getBoundingClientRect = () => ({ top: i * 20, bottom: i * 20 + 20, height: 20, left: 0, right: 100, width: 100, x: 0, y: i * 20, toJSON: () => ({}) });
  });
  document.elementFromPoint = (_x: number, y: number) => target.querySelectorAll("tbody tr")[Math.floor(y / 20)]?.querySelector("td") ?? null;
}

function drag(target: HTMLElement, from: number, toY: number) {
  const body = target.querySelector("tbody")!;
  const cell = target.querySelectorAll("tbody td")[from];
  cell.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: 10, clientY: from * 20 + 10, pointerId: 1 }));
  body.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientX: 10, clientY: toY, pointerId: 1 }));
  const shown = target.querySelector("[data-drop]")?.getAttribute("data-drop");
  body.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, clientX: 10, clientY: toY, pointerId: 1 }));
  return shown;
}

test("Alt and an arrow moves the chosen row", () => {
  const t = table({ rows: tasks, selected: 0, reorder: true });
  const wrap = t.target.querySelector<HTMLElement>(".table-wrap")!;
  expect(wrap.tabIndex).toBe(0);
  wrap.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", altKey: true, bubbles: true }));
  // Without Alt it is no move.
  wrap.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
  expect(t.moves).toEqual([{ row: 0, key: "down" }]);
  t.done();
});

test("a dragged row drops before or after another, with a line where it lands", () => {
  const t = table({ rows: tasks, reorder: true });
  place(t.target);
  expect(drag(t.target, 0, 55)).toBe("after");
  expect(drag(t.target, 2, 21)).toBe("before");
  expect(t.moves).toEqual([
    { row: 0, where: "after", target: 2 },
    { row: 2, where: "before", target: 1 },
  ]);
  expect(t.target.querySelector("[data-drop]")).toBeNull();
  t.done();
});

test("in a tree a drop on the middle of a row puts the row under it", () => {
  const t = table({ rows: tasks, reorder: true, header: false, tree: { parent: "parent", open: "Expand", close: "Collapse", key: "id" } });
  place(t.target);
  expect(drag(t.target, 2, 10)).toBe("into");
  expect(t.moves).toEqual([{ row: 2, where: "into", target: 0 }]);
  t.done();
});

test("in the designer, or without reorder, rows do not move", () => {
  for (const props of [{ reorder: true, design: true }, { reorder: false }]) {
    const t = table({ rows: tasks, selected: 0, ...props });
    place(t.target);
    drag(t.target, 0, 55);
    t.target.querySelector(".table-wrap")!.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", altKey: true, bubbles: true }));
    expect(t.moves).toEqual([]);
    t.done();
  }
});
