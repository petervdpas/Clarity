import { test } from "node:test";
import assert from "node:assert/strict";
import { MIN_WIDTH, readWidths, resized, widthsFor } from "./columnWidths.ts";

test("a column is resized by the drag, never below the narrowest width", () => {
  assert.equal(resized(120, 30), 150);
  assert.equal(resized(120, -200), MIN_WIDTH);
});

test("saved widths are read by column name; anything else reads as none", () => {
  assert.deepEqual(readWidths('{"Field":140,"Type":90.6}'), { Field: 140, Type: 91 });
  assert.deepEqual(readWidths('{"Field":"wide","Type":-3,"Label":200}'), { Label: 200 });
  for (const bad of [null, "", "not json", "[1,2]", "7"]) assert.deepEqual(readWidths(bad), {});
});

test("each column takes its saved width, or the width it was shown at", () => {
  assert.deepEqual(widthsFor(["Field", "Type", ""], [100, 80, 60], { Type: 150, Gone: 9 }), [100, 150, 60]);
});
