// Whether a foldable panel or form is folded, as every runtime says
// (conformance/folding.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { isFolded } from "./folding.ts";

const file = JSON.parse(readFileSync(new URL("../../../conformance/folding.json", import.meta.url), "utf8"));

test("a foldable part is folded as every runtime says", () => {
  assert.ok(file.cases.length > 0);
  for (const c of file.cases) {
    assert.equal(isFolded(c.design, c.part), c.folded, JSON.stringify(c));
  }
});
