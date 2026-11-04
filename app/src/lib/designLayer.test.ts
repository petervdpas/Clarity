// The designer's layer placed by what Clarity.UI reports of the window it drew.
import { test } from "node:test";
import assert from "node:assert/strict";
import { cellsOf, commandsOf, heldBy, layerOf, placeIn } from "./designLayer.ts";
import { chosenOf, filesAddress } from "./clarityWindow.ts";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import type { Frame } from "@clarity/ui/generated/Frame.ts";

const span = (start: number) => ({ start, end: start + 1, line: 1, col: 1 });

function widget(start: number, kind: string, children: Widget[] = [], pages: { start: number; children: Widget[] }[] = []): Widget {
  return {
    props: { kind, name: `${kind}${start}`, col: 1, row: 1, width: 4, height: 1, handlers: [] },
    children,
    pages: pages.map((p) => ({ title: null, columns: null, children: p.children, span: span(p.start) })),
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
