// An accordion as every renderer shows it (conformance/accordions.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pressedSection, shownSection } from "./accordions.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/accordions.json", import.meta.url), "utf8"));

test("the open section shows as on every renderer", () => {
  assert.ok(cases.shown.length > 0);
  for (const c of cases.shown) assert.equal(shownSection(c.open, c.sections), c.shown, JSON.stringify(c));
  assert.equal(shownSection(undefined, 3), 1, "without a state the first section is open");
});

test("a press on a header opens or folds as on every renderer", () => {
  assert.ok(cases.press.length > 0);
  for (const c of cases.press) assert.equal(pressedSection(c.open, c.press, c.sections), c["open after"], JSON.stringify(c));
});
