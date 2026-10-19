// A moment (a datetime field) as a day and a 24-hour time typed as hh:mm,
// the same on the web as on Avalonia.

import { test } from "node:test";
import assert from "node:assert/strict";

import { joinMoment, readTime, splitMoment } from "./moment.ts";

test("a moment shows as its day and its time on a 24-hour clock", () => {
  assert.deepEqual(splitMoment("2026-10-20T09:30:00"), { day: "2026-10-20", time: "09:30" });
  assert.deepEqual(splitMoment("2026-10-20 14:05:59"), { day: "2026-10-20", time: "14:05" });
  assert.deepEqual(splitMoment(null), { day: "", time: "" });
});

test("a typed time is h:mm or hh:mm within a day", () => {
  assert.equal(readTime("9:30"), "09:30");
  assert.equal(readTime(" 14:00 "), "14:00");
  assert.equal(readTime(""), "");
  assert.equal(readTime("24:00"), null);
  assert.equal(readTime("12:60"), null);
  assert.equal(readTime("9.30"), null);
});

test("a day and a time make the moment the backends read; no day is none", () => {
  assert.equal(joinMoment("2026-10-20", "09:30"), "2026-10-20T09:30:00");
  // A day without a time starts at midnight.
  assert.equal(joinMoment("2026-10-20", ""), "2026-10-20T00:00:00");
  assert.equal(joinMoment("", "09:30"), null);
});
