// Lists of values as fields as every renderer edits and shows them
// (conformance/list-fields.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { addTyped, shownList } from "./listFields.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/list-fields.json", import.meta.url), "utf8"));

test("a typed item is added as on every renderer", () => {
  assert.ok(cases.added.length > 0);
  for (const c of cases.added) {
    const got = addTyped(c.item, c.items, c.typed);
    if (c.refused) assert.deepEqual(got, { refused: true }, JSON.stringify(c));
    else assert.deepEqual(got, { items: c.gives }, JSON.stringify(c));
  }
});

test("a list is shown as its items", () => {
  for (const c of cases.shown) {
    const labels: Record<string, string> = c.labels ?? {};
    assert.equal(shownList(c.items, (i) => labels[String(i)] ?? String(i)), c.text, JSON.stringify(c));
  }
});
