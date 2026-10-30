// A canvas's drawing as the web renderer draws it: its shapes as SVG, in
// the canvas's own units (conformance/drawing.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { colorOf, gridEvery, pointIn, pointOf, scrolled, snapped, svgOf, wholeView, zoomed, type Shape, type View } from "./drawing.ts";

type Click = { units: [number, number]; part: [number, number]; at: [number, number]; point: [number, number] | null };

const cases: { shapes: { code: string; sends: Shape }[]; clicks: Click[] } = JSON.parse(
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
  const picture = (name: string) => `resources/${name}`;
  const tags = cases.shapes.map((c) => svgOf(c.sends, { width: 200, height: 100 }, (t) => String(t), picture).tag);
  assert.deepEqual(tags, ["rect", "line", "line", "rect", "circle", "circle", "circle", "text", "text", "image"]);
  // A picture of the resources folder, stretched to fill its box.
  const image = svgOf(cases.shapes[9].sends, { width: 200, height: 100 }, String, picture);
  assert.deepEqual(image.attrs, { x: 10, y: 20, width: 30, height: 15, href: "resources/pill.svg", preserveAspectRatio: "none" });
  // One that is not there draws nothing.
  assert.equal(svgOf(cases.shapes[9].sends, { width: 200, height: 100 }, String, () => null).tag, "g");
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

test("a click on a canvas is where in its units, as every renderer finds it", () => {
  assert.ok(cases.clicks.length > 3);
  for (const c of cases.clicks) {
    const units = { width: c.units[0], height: c.units[1] };
    const got = pointOf(units, { width: c.part[0], height: c.part[1] }, { x: c.at[0], y: c.at[1] });
    assert.deepEqual(got && [got.x, got.y], c.point, JSON.stringify(c));
  }
});

type Pair = [number, number];
type Views = {
  whole: { units: Pair; part: Pair; view: View }[];
  zoomed: { units: Pair; part: Pair; view: View; factor: number; at: Pair; result: View }[];
  scrolled: { view: View; by: Pair; result: View }[];
  points: { units: Pair; view: View; at: Pair; point: Pair | null }[];
};
const views: Views = JSON.parse(readFileSync(new URL("../../../conformance/drawing.json", import.meta.url), "utf8")).views;
const size = ([width, height]: Pair) => ({ width, height });
const at = ([x, y]: Pair) => ({ x, y });

test("a canvas that zooms starts whole, fitted and centred", () => {
  for (const c of views.whole) assert.deepEqual(wholeView(size(c.units), size(c.part)), c.view, JSON.stringify(c));
});

test("a canvas zooms around the pointer, within its scales", () => {
  for (const c of views.zoomed) assert.deepEqual(zoomed(size(c.units), size(c.part), c.view, c.factor, at(c.at)), c.result, JSON.stringify(c));
});

test("a canvas pans by pixels, past its edges too", () => {
  for (const c of views.scrolled) assert.deepEqual(scrolled(c.view, at(c.by)), c.result, JSON.stringify(c));
});

test("a pointer in a zoomed canvas is the unit under it", () => {
  for (const c of views.points) {
    const found = pointIn(size(c.units), c.view, at(c.at));
    assert.deepEqual(found ? [found.x, found.y] : null, c.point, JSON.stringify(c));
  }
});

const grids: {
  lines: { grid: number; scale: number; every: number }[];
  snapped: { value: number; step: number; result: number }[];
} = JSON.parse(readFileSync(new URL("../../../conformance/drawing.json", import.meta.url), "utf8")).grids;

test("a grid thins its dots to 8 pixels apart or more", () => {
  for (const c of grids.lines) assert.equal(gridEvery(c.grid, c.scale), c.every, JSON.stringify(c));
});

test("snapped gives the nearest multiple of its step", () => {
  for (const c of grids.snapped) assert.equal(snapped(c.value, c.step), c.result, JSON.stringify(c));
});
