// The figures of Clarity settings, System, and the status bar's meters.

import { test } from "node:test";
import assert from "node:assert/strict";
import { bytes, ioLevel, kept, percent, HISTORY } from "./systemLoad.ts";

test("sizes are written in bytes, KiB, MiB and GiB", () => {
  assert.equal(bytes(152), "152 bytes");
  assert.equal(bytes(1), "1 byte");
  assert.equal(bytes(1638), "1.6 KiB");
  assert.equal(bytes(28.3 * 1024 * 1024), "28.3 MiB");
  assert.equal(bytes(7.5 * 1024 ** 3), "7.5 GiB");
});

test("a share is written with one decimal", () => {
  assert.equal(percent(4.712), "4.7%");
  assert.equal(percent(100), "100.0%");
});

test("the history keeps the last minute", () => {
  let history: number[] = [];
  for (let i = 0; i < HISTORY + 5; i++) history = kept(history, i);
  assert.equal(history.length, HISTORY);
  assert.equal(history[0], 5);
  assert.equal(history.at(-1), HISTORY + 4);
});

test("the io meter fills on a log scale up to 100 MiB/s", () => {
  assert.equal(ioLevel(0), 0);
  assert.equal(ioLevel(100 * 1024 * 1024), 1);
  assert.equal(ioLevel(10 ** 12), 1);
  const kib = ioLevel(1024);
  const mib = ioLevel(1024 * 1024);
  assert.ok(kib > 0 && kib < mib && mib < 1, `${kib} ${mib}`);
});
