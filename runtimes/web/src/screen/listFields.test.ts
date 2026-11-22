// Lists of values as fields as every renderer edits and shows them
// (conformance/list-fields.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { shownList } from "./listFields.ts";

const cases = JSON.parse(readFileSync(new URL("../../../../conformance/list-fields.json", import.meta.url), "utf8"));

test("a list is shown as its items", () => {
  for (const c of cases.shown) {
    const labels: Record<string, string> = c.labels ?? {};
    assert.equal(shownList(c.items, (i) => labels[String(i)] ?? String(i)), c.text, JSON.stringify(c));
  }
});
