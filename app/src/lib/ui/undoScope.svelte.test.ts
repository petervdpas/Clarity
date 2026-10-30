// Where a part's edits are undone (DESIGN 5.29): the fields of a form or a
// panel share the form's scope, so Ctrl+Z walks back across them; a canvas
// outside one is its own; a part with `no undo` has none.

import { expect, test } from "vitest";
import type { Page } from "../generated/Page";
import { undoScope } from "../pages";

// As the project works them out (clarity_spec::widgets::undo_scope).
const part = (kind: string, name: string, undoScope: string | null, children: unknown[] = []) => ({ props: { kind, name, undoScope }, children, pages: [], span: { start: 0, end: 0, line: 0, col: 0 }, menu: null });

const page = {
  widgets: [part("form", "customerForm", null, [part("textbox", "nameBox", "customerForm"), part("checkbox", "vipBox", "customerForm")]), part("canvas", "diagram", "diagram"), part("textbox", "pinBox", null)],
  frame: { sidebars: [], footer: null },
} as unknown as Page;

test("a form's fields share its scope, a canvas has its own, no undo has none", () => {
  expect(undoScope(page, "nameBox")).toBe("customerForm");
  expect(undoScope(page, "vipBox")).toBe("customerForm");
  expect(undoScope(page, "diagram")).toBe("diagram");
  expect(undoScope(page, "pinBox")).toBeNull();
  expect(undoScope(page, "nowhere")).toBeNull();
});
