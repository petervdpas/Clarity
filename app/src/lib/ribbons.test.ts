// The tab a ribbon shows, as every renderer does (conformance/ribbons.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { shownTab } from "./ribbons.ts";

const ribbons = JSON.parse(readFileSync(new URL("../../../conformance/ribbons.json", import.meta.url), "utf8"));

test("a ribbon shows the tab as every renderer does", () => {
  assert.ok(ribbons.shown.length > 0);
  for (const c of ribbons.shown) {
    assert.equal(shownTab(c.visible, c.tab), c.shown, JSON.stringify(c));
  }
});
