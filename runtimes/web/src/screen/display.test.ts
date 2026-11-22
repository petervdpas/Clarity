// The display part's default digits are the spec's.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { DEFAULT_DIGITS } from "./display.ts";

test("a display has the spec's digits without `digits n`", () => {
  const spec = readFileSync(new URL("../../../../crates/clarity-spec/src/widgets.rs", import.meta.url), "utf8");
  assert.equal(spec.match(/pub const DEFAULT_DIGITS: \w+ = (\d+);/)?.[1], String(DEFAULT_DIGITS));
});
