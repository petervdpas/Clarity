// The chart part on the web (conformance/charts.json): a bar per group, a
// point per group on a line, a slice per group above 0 in the palette's
// colors, each telling its group and number.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import ChartView from "./ChartView.svelte";

// The simulated page measures nothing: the chart draws at size 0, its parts all there.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

const groups = [
  { label: "Open", value: 3, text: "3" },
  { label: "Paid", value: 0, text: "0" },
  { label: "Sent", value: 5, text: "5" },
];

function draw(kind: "bar" | "line" | "pie") {
  const target = document.createElement("div");
  const chart = mount(ChartView, { target, props: { kind, groups } });
  flushSync();
  return { target, done: () => unmount(chart) };
}

test("a bar chart has a bar per group, telling its group and number", () => {
  const { target, done } = draw("bar");
  expect([...target.querySelectorAll("rect.bar title")].map((t) => t.textContent)).toEqual(["Open: 3", "Paid: 0", "Sent: 5"]);
  done();
});

test("a line chart has a point per group", () => {
  const { target, done } = draw("line");
  expect(target.querySelectorAll("circle.point")).toHaveLength(3);
  done();
});

test("a pie has a slice per group above 0, in the palette's colors", () => {
  const { target, done } = draw("pie");
  expect([...target.querySelectorAll("path")].map((p) => p.getAttribute("fill"))).toEqual(["#64748b", "#ef4444"]);
  done();
});
