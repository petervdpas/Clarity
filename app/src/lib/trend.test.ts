// A trend's values (DESIGN 5.26): the last `points` of one number, taken every so many seconds.

import { test } from "node:test";
import assert from "node:assert/strict";
import { numbersOf, sampled, TREND_POINTS } from "./ui/trend.ts";

test("a trend keeps the last points it took", () => {
  let history: number[] = [];
  for (let i = 1; i <= 5; i++) history = sampled(history, i, 3);
  assert.deepEqual(history, [3, 4, 5]);
  assert.equal(TREND_POINTS, 60);
});

test("what is no number is taken as nothing, not as 0", () => {
  assert.deepEqual(sampled([1], null, 3), [1]);
  assert.deepEqual(sampled([1], "2.5", 3), [1, 2.5]);
  assert.deepEqual(sampled([1], "x", 3), [1]);
});

test("a list of numbers shows its last points", () => {
  assert.deepEqual(numbersOf(["1.5", 2, "3"], 2), [2, 3]);
  assert.deepEqual(numbersOf([1, 2], null), [1, 2]);
  assert.deepEqual(numbersOf(null, 5), []);
  assert.deepEqual(numbersOf([1, "x", 3], null), [1, 3]);
});
