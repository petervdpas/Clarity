// A view drawn in a window's content (DESIGN 5.33) from its own file (a
// `.cxaml`): its parts keep their view's keys, so an edit of one goes to the
// view's file in its own places.

import { test } from "vitest";
import { assert } from "vitest";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Preview } from "@clarity/ui/generated/Preview.ts";
import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
import { contentViews, innerPageOf, keyAt, locateKey, placeOf, shownContainers, shownPage } from "@clarity/ui/screen/pages.ts";

const span = (start: number) => ({ start, end: start + 1, line: 1, col: 1 });
const part = (start: number) => ({ props: { kind: "display", col: 1, row: 1, width: 12, height: 3 }, children: [], pages: [], menu: null, levels: [], selects: [], span: span(start) });
const frame = { menu: null, toolbar: null, ribbon: null, sidebars: [], footer: null };
const board = { name: { name: "Board" }, kind: "window", span: span(10), columns: 12, frame, widgets: [{ ...part(50), props: { kind: "content" }, pages: [{ view: "TotalView", children: [], span: span(60), tells: [] }] }, part(120)] } as unknown as Page;
const total = { name: { name: "TotalView" }, kind: "view", span: span(5), columns: 6, frame, widgets: [part(120)] } as unknown as Page;
const files = [{ path: "/p/design/Board.cxaml", pages: [board], apps: [] }, { path: "/p/design/TotalView.cxaml", pages: [total], apps: [] }] as unknown as SpecFile[];
const designKeys = {
  "/p/design/Board.cxaml": { 10: "design/Board.cxaml#Board", 50: "design/Board.cxaml#Board/main", 60: "design/Board.cxaml#Board/main/View[1]", 120: "design/Board.cxaml#Board/note" },
  "/p/design/TotalView.cxaml": { 5: "design/TotalView.cxaml#TotalView", 120: "design/TotalView.cxaml#TotalView/total" },
};
const preview = { project: { files }, designKeys, catalog: [{ kind: "display", container: null }, { kind: "content", container: "pages" }] } as unknown as Preview;

test("a view's part shows in the content with its view's key, beside the window's part at the same place", () => {
  const shown = shownPage(preview, board, files[0]);
  const fromView = shown.page.widgets[0].pages[0].children[0];
  assert.equal(shown.keyOf(fromView), "design/TotalView.cxaml#TotalView/total");
  assert.equal(shown.keyOf(shown.page.widgets[1]), "design/Board.cxaml#Board/note");
  assert.equal(locateKey(preview, shown, "design/TotalView.cxaml#TotalView/total")?.widget, fromView);
  assert.equal(innerPageOf(shown, "design/Board.cxaml#Board/main/View[1]")?.index, 0);
});

test("a key names the file to edit and the place in it", () => {
  const placed = placeOf(preview, "design/TotalView.cxaml#TotalView/total");
  assert.equal(placed?.file.path, "/p/design/TotalView.cxaml");
  assert.equal(placed?.page.name.name, "TotalView");
  assert.equal(placed?.at, 120);
  assert.equal(placeOf(preview, "design/Gone.cxaml#Gone/x"), null);
  assert.equal(keyAt(preview, files[0], 120), "design/Board.cxaml#Board/note");
});

test("a line of a content is its view's grid: a drop there goes into the view", () => {
  const shown = shownPage(preview, board, files[0]);
  assert.equal(contentViews(preview, shown).get("design/Board.cxaml#Board/main/View[1]"), "design/TotalView.cxaml#TotalView");
  const line = shownContainers(preview, shown).find((c) => c.key === "design/TotalView.cxaml#TotalView");
  assert.equal(line?.columns, 6);
});
