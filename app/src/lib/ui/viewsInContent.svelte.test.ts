// A view drawn in a window's content (DESIGN 5.33) from its own file (a
// `.cxaml`): its parts are designer's items of that file, and an edit of
// one goes to the view's file in its own places.

import { test } from "vitest";
import { assert } from "vitest";
import type { Edit } from "@clarity/ui/generated/Edit.ts";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Preview } from "@clarity/ui/generated/Preview.ts";
import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";
import { DESIGN_PLACE, designItem, routedEdit, withViewsShown } from "@clarity/ui/screen/pages.ts";

const span = (start: number) => ({ start, end: start + 1, line: 1, col: 1 });
const part = (start: number) => ({ props: { kind: "display", col: 1, row: 1, width: 12, height: 3 }, children: [], pages: [], menu: null, levels: [], selects: [], span: span(start) });
const board = { name: { name: "Board" }, kind: "window", span: span(10), widgets: [{ ...part(50), props: { kind: "content" }, pages: [{ view: "TotalView", children: [], span: span(60), tells: [] }] }] } as unknown as Page;
const total = { name: { name: "TotalView" }, kind: "view", span: span(5), columns: 12, widgets: [part(120)] } as unknown as Page;
const files = [{ path: "dashboard/Board.cxaml", pages: [board] }, { path: "dashboard/TotalView.cxaml", pages: [total] }] as unknown as SpecFile[];
const preview = { project: { files } } as unknown as Preview;

test("a view of its own file shows in the content with its parts as that file's designer's items", () => {
  const shown = withViewsShown(preview, board);
  const display = shown.widgets[0].pages[0].children[0];
  assert.equal(display.span.start, designItem(1, 120));
});

test("an edit of a view's part drawn in the window goes to the view's file, in its places", () => {
  const edit = { op: "updateWidget", item: designItem(1, 120), container: null, props: {} } as unknown as Edit;
  const routed = routedEdit(preview, files[0], edit);
  assert.equal(routed.file.path, "dashboard/TotalView.cxaml");
  assert.equal((routed.edit as unknown as { item: number }).item, 120);
  assert.equal(routed.base, 2 * DESIGN_PLACE);
  const own = { op: "remove", item: 50 } as unknown as Edit;
  assert.equal(routedEdit(preview, files[0], own).file.path, "dashboard/Board.cxaml");
  const both = { op: "batch", edits: [own, edit] } as unknown as Edit;
  assert.throws(() => routedEdit(preview, files[0], both), /two files/);
});
