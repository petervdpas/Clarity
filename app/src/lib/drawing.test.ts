// A canvas's drawing as the web renderer draws it: its shapes as SVG, in
// the canvas's own units (conformance/drawing.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { colorOf, svgOf, type Shape } from "./drawing.ts";

const cases: { shapes: { code: string; sends: Shape }[] } = JSON.parse(
  readFileSync(new URL("../../../conformance/drawing.json", import.meta.url), "utf8"),
);

test("a color is the theme's by name, or written out", () => {
  assert.equal(colorOf("#101810"), "#101810");
  assert.equal(colorOf("accent"), "var(--accent)");
  assert.equal(colorOf("strong_border"), "var(--border-strong)");
  // A color of the project's theme, else its text color.
  assert.equal(colorOf("lcd_text"), "var(--color-lcd-text, var(--text))");
});

test("each shape the backend sends is an SVG element", () => {
  const tags = cases.shapes.map((c) => svgOf(c.sends, { width: 200, height: 100 }, (t) => String(t)).tag);
  assert.deepEqual(tags, ["rect", "line", "line", "rect", "circle", "circle", "circle", "text", "text"]);
  const line = svgOf(cases.shapes[1].sends, { width: 200, height: 100 }, String);
  assert.deepEqual(line.attrs, { x1: 10, y1: 20, x2: 30, y2: 40, stroke: "var(--accent)", "stroke-width": 3, "stroke-linecap": "round" });
  const clear = svgOf(cases.shapes[0].sends, { width: 200, height: 100 }, String);
  assert.deepEqual(clear.attrs, { x: 0, y: 0, width: 200, height: 100, fill: "#101810" });
  const ring = svgOf(cases.shapes[5].sends, { width: 200, height: 100 }, String);
  assert.deepEqual(ring.attrs, { cx: 50, cy: 50, r: 40, fill: "none", stroke: "var(--border)", "stroke-width": 2 });
  const text = svgOf(cases.shapes[7].sends, { width: 200, height: 100 }, (t) => `shown ${t}`);
  assert.equal(text.text, "shown km/h");
  assert.deepEqual(text.attrs, { x: 50, y: 90, fill: "var(--text-muted)", "font-size": 8, "text-anchor": "middle", "dominant-baseline": "central" });
});
