// The small parts as every renderer shows them (conformance/small-parts.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { linkAddress, progressFull } from "./smallParts.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/small-parts.json", import.meta.url), "utf8"));

test("a link opens what its value says", () => {
  assert.ok(cases.address.length > 0);
  for (const c of cases.address) assert.equal(linkAddress(c.value), c.opens, JSON.stringify(c));
});

test("a progress bar is as full as its value of its max", () => {
  assert.ok(cases.progress.length > 0);
  for (const c of cases.progress) assert.equal(progressFull(c.value, c.max), c.full, JSON.stringify(c));
});
