// Settings as every renderer takes them back (conformance/settings.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fits, keptSettings } from "./settings.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/settings.json", import.meta.url), "utf8"));

test("a kept value fits its setting as on every renderer", () => {
  assert.ok(cases.fits.length > 0);
  for (const c of cases.fits) assert.equal(fits(c.setting, c.value), c.fits, JSON.stringify(c));
});

test("a start takes back only the kept values that still fit", () => {
  for (const c of cases.kept) assert.deepEqual(keptSettings(c.settings, c.kept), c.takes, JSON.stringify(c));
});
