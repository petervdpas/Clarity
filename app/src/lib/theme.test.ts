// Themes in the web renderer (T1). The CSS comes from the generator, one
// file per theme; the renderer holds the classes and says light or dark.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { isDark, pickTheme, styleClass, type ThemeClasses } from "./theme.ts";

type Case = { what: string; dark: boolean; theme: { mode: ThemeClasses["mode"] }; gives: { dark: boolean } };
const cases: { resolves: Case[] } = JSON.parse(readFileSync(new URL("../../../conformance/themes.json", import.meta.url), "utf8"));

test("a theme is light or dark as the conformance cases say", () => {
  for (const c of cases.resolves) assert.equal(isDark(c.theme.mode, c.dark), c.gives.dark, c.what);
});

test("a part holds the class of its style, when the theme has it", () => {
  const theme: ThemeClasses = { mode: "system", class: "theme-shop", styles: [{ name: { name: "primary" }, class: "style-primary" }] };
  assert.equal(styleClass(theme, "primary"), "style-primary");
  assert.equal(styleClass(theme, "missing"), undefined);
  assert.equal(styleClass(null, "primary"), undefined);
  assert.equal(styleClass(theme, null), undefined);
});

test("the theme shown is the one chosen while the application offers it, else the first", () => {
  assert.equal(pickTheme(["Shop", "NightOwl"], "NightOwl"), "NightOwl");
  assert.equal(pickTheme(["Shop", "NightOwl"], undefined), "Shop");
  assert.equal(pickTheme(["Shop"], "Gone"), "Shop");
  assert.equal(pickTheme([], "Shop"), undefined);
});
