// Trees as every renderer shows them (conformance/trees.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { above, shownTree } from "./ui/trees.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/trees.json", import.meta.url), "utf8"));

test("a tree shows its rows as every renderer does", () => {
  for (const c of cases.cases) {
    const shown = shownTree(c.rows, new Set(c.open)).map((s) => [s.place, s.depth, s.children]);
    assert.deepEqual(shown, c.shows, c.name);
  }
});

test("a selected row opens the rows above it", () => {
  for (const c of cases.reveal) {
    const ids = above(c.rows, c.at);
    const order = c.rows.map((r: { id: number }) => r.id).filter((id: number) => ids.includes(id));
    assert.deepEqual(order, c.opens, JSON.stringify(c));
  }
});
