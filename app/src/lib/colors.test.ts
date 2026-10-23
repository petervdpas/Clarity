// The color field type as every renderer reads it (conformance/colors.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { PALETTE, readColor } from "./colors.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/colors.json", import.meta.url), "utf8"));

test("a typed color is read as on every renderer", () => {
  assert.ok(cases.read.length > 0);
  for (const c of cases.read) assert.equal(readColor(c.typed), c.color, JSON.stringify(c));
});

test("the palette is the one every renderer offers", () => {
  assert.deepEqual(PALETTE, cases.palette);
});
