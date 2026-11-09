// The designer's layer placed by what Clarity.UI reports of the window it drew.
import { test } from "node:test";
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { cellsOf, commandsOf, grown, heldBy, layerOf, nudged, placeIn, resizedBy, resizedTo } from "./designLayer.ts";
import { chosenOf, filesAddress, startOptions } from "./clarityWindow.ts";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import type { Frame } from "@clarity/ui/generated/Frame.ts";

const span = (start: number) => ({ start, end: start + 1, line: 1, col: 1 });

function widget(start: number, kind: string, children: Widget[] = [], pages: { start: number; children: Widget[] }[] = []): Widget {
  return {
    props: { kind, name: `${kind}${start}`, col: 1, row: 1, width: 4, height: 1, handlers: [] },
    children,
    pages: pages.map((p) => ({ title: null, columns: null, view: null, tells: [], children: p.children, span: span(p.start) })),
    menu: null,
    span: span(start),
  } as unknown as Widget;
}

const inner = widget(40, "textbox");
const checked = widget(70, "checkbox");
const panel = widget(30, "panel", [inner]);
const tabs = widget(50, "tabs", [], [{ start: 60, children: [checked] }, { start: 80, children: [] }]);
const page = { name: { name: "Main" }, widgets: [widget(20, "button"), panel, tabs], frame: { sidebars: [], footer: null }, span: span(10) } as unknown as Page;

const layout = {
  parts: [
    { item: "20", name: "save", kind: "button", at: [12, 12, 100, 36] as [number, number, number, number] },
    { item: "20", name: "save", kind: "button", at: [500, 500, 1, 1] as [number, number, number, number] },
    { item: "99", name: "gone", kind: "label", at: [0, 0, 1, 1] as [number, number, number, number] },
    { item: null, name: "frameButton", kind: "button", at: [0, 0, 1, 1] as [number, number, number, number] },
    { item: "40", name: "inner", kind: "textbox", at: [20, 60, 80, 36] as [number, number, number, number] },
  ],
  grids: [
    { item: "10", columns: 12, gap: 8, at: [12, 12, 876, 576] as [number, number, number, number], rows: [36, 116, 36] },
    { item: "30", columns: 4, gap: 8, at: [15, 56, 300, 116] as [number, number, number, number], rows: [36] },
    { item: "60", columns: 4, gap: 8, at: [400, 80, 300, 80] as [number, number, number, number], rows: [36] },
    { item: null, columns: 3, gap: 8, at: [0, 0, 10, 10] as [number, number, number, number], rows: [36] },
  ],
};

test("a part shows once, by its item, and only when the design has it", () => {
  const layer = layerOf(layout, page);
  assert.deepEqual(
    layer.parts.map((p) => [p.item, p.box.x, p.box.y]),
    [
      [20, 12, 12],
      [40, 20, 60],
    ],
  );
});

test("each grid knows the containers it lies in, so a part is never dropped into itself", () => {
  const layer = layerOf(layout, page);
  const by = (item: number) => layer.grids.find((g) => g.item === item);
  assert.deepEqual(by(10)?.inside, []);
  assert.deepEqual(by(30)?.inside, [30]);
  assert.deepEqual(by(60)?.inside, [50]);
  assert.equal(layer.grids.length, 3, "a grid of no design item is left out");
  assert.deepEqual(heldBy(tabs), [50, 60, 80, 70]);
});

test("cells and a drop place follow the rows as Clarity.UI laid them out", () => {
  const grid = layerOf(layout, page).grids[0];
  const cells = cellsOf(grid);
  assert.equal(cells.length, 36);
  const column = (876 - 8 * 11) / 12;
  assert.deepEqual(cells[12], { x: 0, y: 44, width: column, height: 116 });
  // Two rows from the second: the tall one, the gap and the third.
  assert.deepEqual(placeIn(grid, { col: 2, row: 2, width: 3, height: 2 }), { x: column + 8, y: 44, width: 3 * column + 16, height: 116 + 8 + 36 });
  // Past the last row, rows are 36 high.
  assert.equal(placeIn(grid, { col: 1, row: 5, width: 1, height: 1 }).y, 36 + 8 + 116 + 8 + 36 + 8 + 36 + 8);
});

test("below the last part the cells go on to the bottom of the grid, at the row height given", () => {
  // A page's own exact grid: 10 pixel rows, no gaps, the window 290 high and its parts reaching 230.
  const grid = { item: 1, columns: 2, gap: 0, box: { x: 0, y: 0, width: 20, height: 290 }, rows: Array(23).fill(10), inside: [] };
  assert.equal(cellsOf(grid).length, 2 * 23, "without a row height, only the rows laid out");
  const cells = cellsOf(grid, 10);
  assert.equal(cells.length, 2 * 29, "290 pixels of 10 pixel rows");
  assert.deepEqual(cells.at(-1), { x: 10, y: 280, width: 10, height: 10 });
});

test("past the last row, a drop place follows the project's row height", () => {
  const grid = layerOf(layout, page).grids[0];
  assert.equal(placeIn(grid, { col: 1, row: 5, width: 1, height: 2 }, 30).y, 36 + 8 + 116 + 8 + 36 + 8 + 30 + 8);
  assert.equal(placeIn(grid, { col: 1, row: 5, width: 1, height: 2 }, 30).height, 30 + 8 + 30);
});

// conformance/grid-layout.json: where the designer places a part on a grid
// of rows it does not grow is where Clarity.UI lays it, at every row height.
test("a drop place is where every Clarity.UI lays a part out", () => {
  type Case = { what: string; columns: number; width: number; gap: number; row_height?: number; parts: { col: number; row: number; width: number; height: number; grows: boolean; bottom?: boolean }[]; gives: { boxes: number[][] } };
  const file = JSON.parse(readFileSync(new URL("../../../conformance/grid-layout.json", import.meta.url), "utf8")) as { row_height: number; cases: Case[] };
  // A part marked bottom moves with the height to spare, which the drop place does not know.
  const kept = file.cases.filter((c) => c.parts.length > 0 && !c.parts.some((p) => p.grows || p.bottom));
  assert.ok(kept.some((c) => (c.row_height ?? file.row_height) !== 36), "a case at another row height");
  for (const c of kept) {
    const grid = { item: 1, columns: c.columns, gap: c.gap, box: { x: 0, y: 0, width: c.width, height: 0 }, rows: [], inside: [] };
    c.parts.forEach((p, i) => {
      const box = placeIn(grid, p, c.row_height ?? file.row_height);
      const want = c.gives.boxes[i];
      for (const [k, v] of [box.x, box.y, box.width, box.height].entries()) assert.ok(Math.abs(v - want[k]) < 0.001, `${c.what}: part ${i + 1} ${JSON.stringify(box)}`);
    });
  }
});

test("a frame's buttons are known by the window's own frame or its app's, a container's pages and a table's columns come along", () => {
  const command = (start: number, items: unknown[] = []) => ({ props: {}, items, span: span(start) });
  const framed = { ...page, frame: { menu: { name: null, items: [command(200, [command(210)])], span: span(199) }, toolbar: null, ribbon: null, sidebars: [], footer: null } } as unknown as Page;
  const app = { menu: null, toolbar: { name: null, items: [command(300)], span: span(299) }, ribbon: null, sidebars: [], footer: null } as unknown as Frame;
  assert.deepEqual(commandsOf(framed.frame), [200, 210]);
  const laid = {
    parts: [
      { item: "200", name: "fileItem", kind: "command", at: [0, 0, 40, 20] as [number, number, number, number] },
      { item: "300", name: "tool", kind: "command", at: [50, 0, 40, 20] as [number, number, number, number] },
      { item: "400", name: "stranger", kind: "command", at: [90, 0, 40, 20] as [number, number, number, number] },
      { item: "50", name: "tabs", kind: "tabs", at: [0, 40, 300, 200] as [number, number, number, number], pages: [{ item: "60", title: "One", at: [0, 40, 60, 24] as [number, number, number, number] }, { item: "80", title: "Two", at: null }] },
    ],
    grids: [],
  };
  const layer = layerOf(laid, framed, app);
  assert.deepEqual(
    layer.parts.map((p) => [p.item, p.kind, p.own]),
    [
      [200, "command", true],
      [300, "command", false],
      [50, "tabs", true],
    ],
  );
  assert.deepEqual(layer.parts[2].pages, [
    { item: 60, title: "One", box: { x: 0, y: 40, width: 60, height: 24 } },
    { item: 80, title: "Two", box: null },
  ]);
});

test("the IDE's language is the drawing's choice", () => {
  assert.deepEqual(chosenOf(undefined, "en"), {});
  assert.deepEqual(chosenOf(null, "en"), { language: "en" });
  assert.deepEqual(chosenOf("nl", "en"), { language: "nl" });
  assert.equal(
    filesAddress("http://x/s/", { window: "Main" }, true, 4, { pages: { 50: 1, 70: 0 }, draft: true, open: ["pick"] }),
    "http://x/s/files.json?window=Main&design=1&pages=50.1,70.0&draft=1&open=pick&revision=4",
  );
});

test("the preview takes input and has the accessible mirror; the design canvas neither", () => {
  const preview = startOptions("http://x/s/", { window: "Main" }, false, 1);
  assert.equal(preview.input, true);
  assert.equal(preview.access, true);
  const design = startOptions("http://x/s/", { window: "Main" }, true, 1);
  assert.equal(design.input, false);
  assert.equal(design.access, false);
});

test("an arrow key moves a part one cell, never out of its grid", () => {
  const at = { col: 2, row: 3, width: 4, height: 2 };
  assert.deepEqual(nudged(at, 12, "ArrowLeft"), { col: 1, row: 3 });
  assert.deepEqual(nudged(at, 12, "ArrowRight"), { col: 3, row: 3 });
  assert.deepEqual(nudged(at, 12, "ArrowUp"), { col: 2, row: 2 });
  assert.deepEqual(nudged(at, 12, "ArrowDown"), { col: 2, row: 4 });
  assert.equal(nudged({ ...at, col: 1 }, 12, "ArrowLeft"), null, "the first column is the edge");
  assert.equal(nudged({ ...at, col: 9 }, 12, "ArrowRight"), null, "its last column on the grid's last");
  assert.equal(nudged({ ...at, row: 1 }, 12, "ArrowUp"), null);
  assert.equal(nudged(at, 12, "Enter"), null, "only the arrows");
});

test("shift and an arrow key resize a part one cell, never below one or out of its grid", () => {
  const at = { col: 2, row: 3, width: 4, height: 2 };
  assert.deepEqual(grown(at, 12, "ArrowRight"), { width: 5, height: 2 });
  assert.deepEqual(grown(at, 12, "ArrowLeft"), { width: 3, height: 2 });
  assert.deepEqual(grown(at, 12, "ArrowDown"), { width: 4, height: 3 });
  assert.deepEqual(grown(at, 12, "ArrowUp"), { width: 4, height: 1 });
  assert.equal(grown({ ...at, width: 1 }, 12, "ArrowLeft"), null, "one cell is the least");
  assert.equal(grown({ ...at, height: 1 }, 12, "ArrowUp"), null);
  assert.equal(grown({ ...at, col: 9 }, 12, "ArrowRight"), null, "its last column on the grid's last");
  assert.equal(grown(at, 12, "Tab"), null);
});

test("a line of a window's content is named by the view it shows", () => {
  const content = widget(50, "content", [], [{ start: 60, children: [] }, { start: 80, children: [] }]);
  content.pages[0].view = "StartView";
  content.pages[1].view = "ProjectEditor";
  const shown = { ...page, widgets: [content] } as unknown as Page;
  const laid = {
    parts: [{ item: "50", name: "main", kind: "content", at: [0, 0, 300, 200] as [number, number, number, number], pages: [{ item: "60", title: null, at: null }, { item: "80", title: null, at: null }] }],
    grids: [],
  };
  const layer = layerOf(laid, shown);
  assert.deepEqual(layer.parts[0].pages.map((p) => p.title), ["StartView", "ProjectEditor"]);
});

test("the grid of a line of a window's content is its view's, so the view's parts resize and move on it", () => {
  const content = widget(50, "content", [], [{ start: 60, children: [] }]);
  content.pages[0].view = "TotalView";
  const shown = { ...page, widgets: [content] } as unknown as Page;
  const laid = { parts: [], grids: [{ item: "60", columns: 12, gap: 8, at: [0, 0, 300, 200] as [number, number, number, number], rows: [36, 36] }] };
  assert.equal(layerOf(laid, shown, null, new Map([[60, 900]])).grids[0].item, 900);
  assert.equal(layerOf(laid, shown).grids[0].item, 60, "a line without a view in the file keeps its own");
});

test("a resize follows how far the handle moved, in whole cells, not the cell under the pointer", () => {
  assert.deepEqual(resizedBy({ x: 100, y: 100 }, { x: 104, y: 107 }, 60, 40), { columns: 0, rows: 0 }, "a small move at the start keeps the size");
  assert.deepEqual(resizedBy({ x: 100, y: 100 }, { x: 40, y: 190 }, 60, 40), { columns: -1, rows: 2 });
  const at = { col: 1, width: 4, height: 7 };
  assert.deepEqual(resizedTo(at, { columns: 0, rows: 2 }, 12), { width: 4, height: 9 }, "past the last row");
  assert.deepEqual(resizedTo(at, { columns: -9, rows: -9 }, 12), { width: 1, height: 1 });
  assert.deepEqual(resizedTo(at, { columns: 20, rows: 0 }, 12), { width: 12, height: 7 }, "not past the last column");
});
