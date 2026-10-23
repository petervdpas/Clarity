// Popovers and context menu items as every renderer shows them (conformance/overlays.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { itemClicks, itemShown, popoverShown } from "./overlays.ts";

const overlays = JSON.parse(readFileSync(new URL("../../../conformance/overlays.json", import.meta.url), "utf8"));

test("a popover shows once opened", () => {
  assert.ok(overlays.popover.length > 0);
  for (const c of overlays.popover) assert.equal(popoverShown(c.part), c.shown, JSON.stringify(c));
});

test("a context menu's items show and take clicks as their part state says", () => {
  assert.ok(overlays.item.length > 0);
  for (const c of overlays.item) {
    assert.equal(itemShown(c.part), c.shown, JSON.stringify(c));
    assert.equal(itemClicks(c.part), c.clicks, JSON.stringify(c));
  }
});
