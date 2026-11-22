// The range of a slider or a spin box as the designer shows it.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { numberRange } from "./ranges.ts";

test("the range is as written, else the field's own; a slider runs from 0 to 100", () => {
  const number = (value: string) => ({ kind: "number" as const, value });
  const slider = { kind: "slider", min: null, max: null, step: null };
  assert.deepEqual(numberRange(slider), { min: "0", max: "100", step: "1" });
  assert.deepEqual(numberRange(slider, { min: number("1"), max: number("99") }), { min: "1", max: "99", step: "1" });
  assert.deepEqual(numberRange({ kind: "spinbox", min: "-10", max: null, step: "0.5" }, { min: number("1"), max: null }), { min: "-10", max: null, step: "0.5" });
  assert.deepEqual(numberRange({ kind: "spinbox", min: null, max: null, step: null }), { min: null, max: null, step: "1" });
});

test("a range's defaults are as the cases the spec runs say (conformance/ranges.json, defaults)", () => {
  const cases = JSON.parse(readFileSync(new URL("../../../../conformance/ranges.json", import.meta.url), "utf8"));
  const number = (value: string | null) => (value === null ? null : { kind: "number" as const, value });
  for (const c of cases.defaults) {
    const field = c.field ? { min: number(c.field.min), max: number(c.field.max) } : undefined;
    assert.deepEqual(numberRange({ kind: c.kind, ...c.written }, field), c.range, JSON.stringify(c));
  }
});
