// Which grid row a point is in, when rows that grow are taller than the
// others: the designer places parts by it, on a window laid out as it runs.

import { test } from "node:test";
import assert from "node:assert/strict";

import { GAP, ROW_HEIGHT, rowIn, windowSize } from "./grid.ts";

test("a point is in the row whose height it falls in, gaps counting to the row above", () => {
  const tracks = [ROW_HEIGHT, 200, ROW_HEIGHT];
  assert.equal(rowIn(tracks, 0), 1);
  assert.equal(rowIn(tracks, ROW_HEIGHT + GAP - 1), 1);
  assert.equal(rowIn(tracks, ROW_HEIGHT + GAP), 2);
  // The tall row that grows.
  assert.equal(rowIn(tracks, ROW_HEIGHT + GAP + 199), 2);
  assert.equal(rowIn(tracks, ROW_HEIGHT + GAP + 200 + GAP), 3);
});

test("below the last row, rows of the usual height follow", () => {
  const tracks = [ROW_HEIGHT, ROW_HEIGHT];
  const end = 2 * (ROW_HEIGHT + GAP);
  assert.equal(rowIn(tracks, end), 3);
  assert.equal(rowIn(tracks, end + ROW_HEIGHT + GAP), 4);
  assert.equal(rowIn([], 0), 1);
});

// A container's parts sit on its own grid, as many columns as it is wide:
// resizing it scales its parts across with it, so its content keeps its shape.

import { resizeEdits, scaledColumns } from "./grid.ts";
import type { Widget } from "./generated/Widget.ts";
import type { WidgetProps } from "./generated/WidgetProps.ts";

function part(kind: string, col: number, width: number, start: number, children: Widget[] = []): Widget {
  const props = { kind, col, row: 1, width, height: 1 } as unknown as WidgetProps;
  return { props, children, pages: [], span: { start, end: start + 10, line: 1, col: 1 } } as Widget;
}

test("columns scale by their edges, so parts keep their order and stay apart", () => {
  assert.deepEqual(scaledColumns(1, 2, 6, 12), { col: 1, width: 4 });
  assert.deepEqual(scaledColumns(3, 4, 6, 12), { col: 5, width: 8 });
  assert.deepEqual(scaledColumns(5, 2, 12, 6), { col: 3, width: 1 });
  // Never narrower than one column.
  assert.deepEqual(scaledColumns(1, 1, 12, 4), { col: 1, width: 1 });
});

test("resizing a panel scales what is inside it, all in one batch of edits", () => {
  const inner = part("panel", 3, 4, 300, [part("textbox", 1, 2, 400)]);
  const panel = part("panel", 1, 6, 100, [part("label", 1, 2, 200), inner]);
  const edits = resizeEdits(panel, 12, 1);
  const moved = edits.map((e) => (e.op === "updateWidget" ? [e.item, e.props.col, e.props.width] : null));
  assert.deepEqual(moved, [
    [100, 1, 12],
    [200, 1, 4],
    [300, 5, 8],
    // The panel inside grew too, so its own part scales with it.
    [400, 1, 4],
  ]);
  // A height change alone moves nothing inside.
  assert.equal(resizeEdits(panel, 6, 3).length, 1);
});

test("a window opens at its own size, or at the standard one", () => {
  assert.deepEqual(windowSize({ size: { width: 320, height: 460 } }), { width: 320, height: 460 });
  assert.deepEqual(windowSize({ size: null }), { width: 1100, height: 760 });
});
