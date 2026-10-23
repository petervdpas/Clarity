// The dropdown's options as every renderer offers them (conformance/dropdown-options.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { searched, suggestions } from "./dropdownOptions.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/dropdown-options.json", import.meta.url), "utf8"));

test("a typed dropdown offers the values its field has", () => {
  assert.ok(cases.suggestions.length > 0);
  for (const c of cases.suggestions) assert.deepEqual(suggestions(c.values, c.typed), c.gives, JSON.stringify(c));
});

test("a search dropdown offers the records that hold every typed word", () => {
  for (const c of cases.search.cases) {
    assert.deepEqual(
      searched(cases.search.options, c.typed).map((o) => o.id),
      c.gives,
      JSON.stringify(c),
    );
  }
});
