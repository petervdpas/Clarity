// The version a field writes into the project file.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { VERSION_PATTERN, versionWritten } from "./version.ts";

test("a version field keeps a version, clears an empty one, and saves nothing else", () => {
  assert.equal(versionWritten(" 1.2.0 "), "1.2.0");
  assert.equal(versionWritten("2.0.0-beta.1"), "2.0.0-beta.1");
  assert.equal(versionWritten(""), null);
  for (const wrong of ["1.2", "v1.2.0", "1.2.x", "1.2.0-"]) assert.equal(versionWritten(wrong), undefined, wrong);
});

test("a version is written as the cases the project file's reader runs say (conformance/app-version.json)", () => {
  const cases = JSON.parse(readFileSync(new URL("../../../conformance/app-version.json", import.meta.url), "utf8"));
  for (const c of cases.cases) assert.equal(VERSION_PATTERN.test(c.text), c.is, c.text);
});
