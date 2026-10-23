// Charts as every renderer draws them (conformance/charts.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { groupsOf, scaleOf, sliceColors } from "./ui/charts.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/charts.json", import.meta.url), "utf8"));

test("rows group, count, sum and average as on every renderer", () => {
  for (const c of cases.groups) assert.deepEqual(groupsOf(c.rows, c.by, c.measure, c.order ?? []), c.gives, c.name);
});

test("the axis takes its steps as on every renderer", () => {
  for (const c of cases.scale) assert.deepEqual(scaleOf(c.low, c.high), { bottom: c.bottom, top: c.top, step: c.step }, JSON.stringify(c));
});

test("a pie's slices take their colors as on every renderer", () => {
  for (const c of cases.slices) assert.deepEqual(sliceColors(c.values), c.colors);
});
