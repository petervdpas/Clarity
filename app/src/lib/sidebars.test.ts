// A sidebar's width as every renderer gives it (conformance/sidebars.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { sidebarWidth } from "./sidebars.ts";

const file = JSON.parse(readFileSync(new URL("../../../conformance/sidebars.json", import.meta.url), "utf8"));

test("a sidebar is dragged within its limits as every renderer does", () => {
  assert.ok(file.widths.length > 0);
  for (const c of file.widths) {
    assert.equal(sidebarWidth(c.dragged, c.min, c.max), c.width, JSON.stringify(c));
  }
});

