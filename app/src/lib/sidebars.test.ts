// A sidebar's width as every renderer gives it (conformance/sidebars.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { sidebarNow, sidebarWidth } from "./sidebars.ts";

const file = JSON.parse(readFileSync(new URL("../../../conformance/sidebars.json", import.meta.url), "utf8"));

test("a sidebar is dragged within its limits as every renderer does", () => {
  assert.ok(file.widths.length > 0);
  for (const c of file.widths) {
    assert.equal(sidebarWidth(c.dragged, c.min, c.max), c.width, JSON.stringify(c));
  }
});


test("a sidebar shows as designed until the person folds or fills it", () => {
  const props = { collapsible: true, collapsed: true, maximizable: true };
  assert.deepEqual(sidebarNow(props, undefined, undefined), { collapsed: true, maximized: false });
  // Unfolded on the canvas, it stays so while its design is the same.
  assert.deepEqual(sidebarNow(props, undefined, { collapsed: false, designed: true }), { collapsed: false, maximized: false });
  // Ticking or unticking "starts folded" shows the new design again.
  assert.deepEqual(sidebarNow({ ...props, collapsed: false }, undefined, { collapsed: true, designed: true }), { collapsed: false, maximized: false });
  // A handler's state wins over the person's.
  assert.deepEqual(sidebarNow(props, { collapsed: false, maximized: true }, { collapsed: true, designed: true }), { collapsed: false, maximized: true });
  // What it cannot do, it does not.
  assert.deepEqual(sidebarNow({ collapsible: false, collapsed: true, maximizable: false }, undefined, { collapsed: true, maximized: true, designed: true }), { collapsed: false, maximized: false });
});
