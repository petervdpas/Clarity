// The display part's text and segments, against the cases every renderer
// shares (conformance/display.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { SEGMENTS, cells, displayText } from "./display.ts";

type Cases = {
  shows: { value: string; digits: number; text: string }[];
  cells: { text: string; digits: number; cells: [string, boolean][] }[];
  segments: Record<string, string>;
};
const cases: Cases = JSON.parse(readFileSync(new URL("../../../conformance/display.json", import.meta.url), "utf8"));

test("a value shows rounded to fit its digits, or as E", () => {
  for (const c of cases.shows) assert.equal(displayText(c.value, c.digits), c.text, `${c.value} in ${c.digits}`);
});

test("the text fills the cells from the right, a point lit on its digit", () => {
  for (const c of cases.cells) assert.deepEqual(cells(c.text, c.digits), c.cells.map(([char, point]) => ({ char, point })), c.text);
});

test("each character lights its segments", () => {
  assert.deepEqual(SEGMENTS, cases.segments);
});
