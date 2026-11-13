// The lines the theme form writes, as the spec reads them.
import { test } from "node:test";
import assert from "node:assert/strict";
import { EMPTY_LOOK, colorLine, fontLine, longHex, lookText, partLine, radiusLine, settingLine, sizeLine, statesOf, styleLine, titleLine } from "./themeLines.ts";

test("a color is written with its dark value only when it differs", () => {
  assert.deepEqual(colorLine("accent", "#2f6fdf", "#5b8cff"), { key: "color accent", line: 'color accent "#2f6fdf" dark "#5b8cff"' });
  assert.deepEqual(colorLine("accent", "#2f6fdf", "#2f6fdf"), { key: "color accent", line: 'color accent "#2f6fdf"' });
  assert.deepEqual(colorLine("accent", "#2f6fdf", null), { key: "color accent", line: 'color accent "#2f6fdf"' });
});

test("the text font, size and the radius of all are written without their role", () => {
  assert.deepEqual(fontLine("text", "Inter"), { key: "font text", line: 'font "Inter"' });
  assert.deepEqual(fontLine("heading", "Georgia"), { key: "font heading", line: 'font heading "Georgia"' });
  assert.deepEqual(fontLine("mono", " "), { key: "font mono", line: null });
  assert.deepEqual(sizeLine("text", 14), { key: "size text", line: "size 14" });
  assert.deepEqual(sizeLine("small", 12), { key: "size small", line: "size small 12" });
  assert.deepEqual(radiusLine("all", 6), { key: "radius all", line: "radius 6" });
  assert.deepEqual(radiusLine("input", null), { key: "radius input", line: null });
  assert.deepEqual(settingLine("mode", "dark"), { key: "mode", line: "mode dark" });
  assert.deepEqual(settingLine("padding", null), { key: "padding", line: null });
  assert.deepEqual(titleLine("Night owl"), { key: "title", line: 'title "Night owl"' });
});

test("a part's look is written in the theme's order, and an empty look takes the line away", () => {
  const look = { ...EMPTY_LOOK, background: "raised", text: "#fff", border: "none", weight: "bold" as const, size: 12 };
  assert.equal(lookText(look), 'background raised, text "#fff", border none, weight bold, size 12');
  assert.deepEqual(partLine("button", "hover", look), { key: "part button hover", line: 'part button hover background raised, text "#fff", border none, weight bold, size 12' });
  assert.deepEqual(partLine("button", null, EMPTY_LOOK), { key: "part button", line: null });
  assert.deepEqual(styleLine("primary", EMPTY_LOOK), { key: "style primary", line: "style primary border none" });
});

test("only the parts with rows to choose have a selected look", () => {
  const states = ["hover", "pressed", "focus", "selected"];
  assert.deepEqual(statesOf("button", states, ["list", "tab"]), [null, "hover", "pressed", "focus"]);
  assert.deepEqual(statesOf("tab", states, ["list", "tab"]), [null, "hover", "pressed", "focus", "selected"]);
  assert.equal(longHex("#abc"), "#aabbcc");
  assert.equal(longHex("#aabbcc"), "#aabbcc");
});
