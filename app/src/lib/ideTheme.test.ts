import { test } from "node:test";
import assert from "node:assert/strict";
import { IDE_THEMES, schemeOf } from "./ideTheme.ts";

test("each theme of Clarity says whether the page is light or dark", () => {
  assert.deepEqual(
    IDE_THEMES.map((t) => t.value),
    ["system", "light", "dark", "cream-green"],
  );
  assert.equal(schemeOf("system", true), "dark");
  assert.equal(schemeOf("system", false), "light");
  assert.equal(schemeOf("dark", false), "dark");
  assert.equal(schemeOf("light", true), "light");
  // Cream and green is paper: light, whatever the system says.
  assert.equal(schemeOf("cream-green", true), "light");
});
