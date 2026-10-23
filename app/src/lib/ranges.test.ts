// The slider and the spin box as every renderer moves them (conformance/ranges.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dragged, keyed, numberRange, rangeValue, typedIn } from "./ranges.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/ranges.json", import.meta.url), "utf8"));

test("a key moves the value a step, ten, or to an end, within the range", () => {
  assert.ok(cases.keys.length > 0);
  for (const c of cases.keys) assert.equal(keyed(c.value, c, c.key), c.after, JSON.stringify(c));
});

test("a slider takes the nearest step where it is dragged to", () => {
  assert.ok(cases.drag.length > 0);
  for (const c of cases.drag) assert.equal(dragged(c.at, { ...c, step: c.step }), c.value, JSON.stringify(c));
});

test("a typed number is kept within the range", () => {
  assert.ok(cases.typed.length > 0);
  for (const c of cases.typed) assert.equal(typedIn(c.value, { ...c, step: "1" }), c.after, JSON.stringify(c));
});

test("a whole number is a number, a decimal stays text", () => {
  assert.equal(rangeValue("12", true), 12);
  assert.equal(rangeValue("12.5", false), "12.5");
  assert.equal(keyed(4, { min: "0", max: "10", step: "1" }, "up"), "5", "a whole number comes as a number");
});

test("the range is as written, else the field's own; a slider runs from 0 to 100", () => {
  const number = (value: string) => ({ kind: "number" as const, value });
  const slider = { kind: "slider", min: null, max: null, step: null };
  assert.deepEqual(numberRange(slider), { min: "0", max: "100", step: "1" });
  assert.deepEqual(numberRange(slider, { min: number("1"), max: number("99") }), { min: "1", max: "99", step: "1" });
  assert.deepEqual(numberRange({ kind: "spinbox", min: "-10", max: null, step: "0.5" }, { min: number("1"), max: null }), { min: "-10", max: null, step: "0.5" });
  assert.deepEqual(numberRange({ kind: "spinbox", min: null, max: null, step: null }), { min: null, max: null, step: "1" });
});
