// Reordering as every renderer does it (conformance/reorder.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { reorder } from "./ui/reorder.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/reorder.json", import.meta.url), "utf8"));

test("a list's rows move as every renderer moves them", () => {
  for (const c of cases.lists) {
    const moved = reorder(c.rows, c.move, false);
    if (c.refused) assert.equal(moved, null, c.name);
    else assert.deepEqual(moved && { order: moved.order, numbers: moved.numbers }, { order: c.order, numbers: c.numbers }, c.name);
  }
});

test("a tree's rows move as every renderer moves them", () => {
  for (const c of cases.trees) {
    const moved = reorder(c.rows, c.move, true);
    if (c.refused) assert.equal(moved, null, c.name);
    else assert.deepEqual(moved, { order: c.order, parents: c.parents, numbers: c.numbers }, c.name);
  }
});
