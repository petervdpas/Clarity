// A canvas the person zooms and drags on, as Clarity's own diagram designer
// moves (conformance/drawing.json, views): a drag runs its handler one call
// at a time with the latest place and where it was pressed, the drop after
// the drags, and a press that does not move is a click. A zoomed view stays
// when the drawing changes.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import type { Pointer } from "../live";
import CanvasView from "./CanvasView.svelte";

// The simulated page lays nothing out: every part is 400 x 200 pixels, and
// a part's size is told at once, as a browser tells it when it is shown.
Object.defineProperty(HTMLElement.prototype, "clientWidth", { configurable: true, get: () => 400 });
Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => 200 });
globalThis.ResizeObserver = class {
  constructor(private told: ResizeObserverCallback) {}
  observe(target: Element) {
    this.told([{ target, contentRect: { width: 400, height: 200 } } as unknown as ResizeObserverEntry], this as unknown as ResizeObserver);
  }
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

/** The view a canvas shows: its viewBox, x, y, width and height in units. */
const shownBox = (svg: SVGSVGElement) => svg.getAttribute("viewBox")!.split(" ").map((v) => Math.round(Number(v) * 100) / 100);

/** A pointer event as a browser sends it, at a place in the part. */
function pointer(type: string, x: number, y: number): MouseEvent {
  const e = new MouseEvent(type, { clientX: x, clientY: y, button: 0, bubbles: true, cancelable: true });
  Object.defineProperty(e, "pointerId", { value: 1 });
  Object.defineProperty(e, "isPrimary", { value: true });
  return e;
}

/** A canvas of 1000 x 500 units in a part of 400 x 200 pixels: 2.5 units a pixel. */
function canvas(props: Record<string, unknown>) {
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(CanvasView, { target: holder, props: { drawing: [], units: { width: 1000, height: 500 }, shown: String, ...props } });
  flushSync();
  const svg = holder.querySelector("svg")!;
  svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 400, height: 200, right: 400, bottom: 200, x: 0, y: 0, toJSON: () => ({}) });
  return { holder, part, svg, done: () => (unmount(part), holder.remove()) };
}

test("a drag runs one call at a time with the latest place, then the drop", async () => {
  const drags: Pointer[] = [];
  const drops: Pointer[] = [];
  const clicks: Pointer[] = [];
  let finish: () => void = () => {};
  const { svg, done } = canvas({
    ondrag: (p: Pointer) => {
      drags.push(p);
      return new Promise<void>((resolve) => (finish = resolve));
    },
    ondrop: (p: Pointer) => drops.push(p),
    onpoint: (p: Pointer) => clicks.push(p),
  });
  svg.dispatchEvent(pointer("pointerdown", 40, 20));
  svg.dispatchEvent(pointer("pointermove", 60, 20));
  svg.dispatchEvent(pointer("pointermove", 80, 20));
  svg.dispatchEvent(pointer("pointermove", 100, 40));
  // The first runs; of the two that came while it ran, only the latest waits.
  expect(drags).toEqual([{ x: 150, y: 50, fromX: 100, fromY: 50 }]);
  finish();
  await Promise.resolve();
  await Promise.resolve();
  expect(drags.at(-1)).toEqual({ x: 250, y: 100, fromX: 100, fromY: 50 });
  expect(drags.length).toBe(2);
  svg.dispatchEvent(pointer("pointerup", 100, 40));
  expect(drops).toEqual([]);
  finish();
  await new Promise((r) => setTimeout(r, 0));
  expect(drops).toEqual([{ x: 250, y: 100, fromX: 100, fromY: 50 }]);
  expect(clicks).toEqual([]);
  done();
});

test("a press that does not move is a click", () => {
  const clicks: Pointer[] = [];
  const { svg, done } = canvas({ onpoint: (p: Pointer) => clicks.push(p), ondrag: () => Promise.resolve() });
  svg.dispatchEvent(pointer("pointerdown", 40, 20));
  svg.dispatchEvent(pointer("pointermove", 41, 21));
  svg.dispatchEvent(pointer("pointerup", 41, 21));
  expect(clicks).toEqual([{ x: 102, y: 52 }]);
  done();
});

test("the wheel zooms around the pointer and pans, the buttons zoom, and a redraw keeps the view", () => {
  const props = $state({ drawing: [] as never[], units: { width: 1000, height: 500 }, shown: String, zoom: true });
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(CanvasView, { target: holder, props });
  flushSync();
  const svg = holder.querySelector("svg")!;
  svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 400, height: 200, right: 400, bottom: 200, x: 0, y: 0, toJSON: () => ({}) });
  // Whole: 0.4 pixels a unit, the canvas filling the part.
  expect(shownBox(svg)).toEqual([0, 0, 1000, 500]);
  const level = holder.querySelector(".level")!;
  expect(level.textContent).toBe("40%");
  // Ctrl and a notch of the wheel zoom in 15% around the pointer: the unit under it, 500, 250, stays.
  svg.dispatchEvent(new WheelEvent("wheel", { deltaY: -100, ctrlKey: true, clientX: 200, clientY: 100, cancelable: true }));
  flushSync();
  expect(level.textContent).toBe("46%");
  const [x, y] = shownBox(svg);
  expect([Math.round(x + 200 / 0.46), Math.round(y + 100 / 0.46)]).toEqual([500, 250]);
  // The wheel alone pans: 46 pixels down is 100 units at 0.46 pixels a unit.
  svg.dispatchEvent(new WheelEvent("wheel", { deltaY: 46, clientX: 200, clientY: 100, cancelable: true }));
  flushSync();
  expect(Math.round(shownBox(svg)[1] - y)).toBe(100);
  // A new drawing, and new units of the same size, keep the view.
  props.drawing = [];
  props.units = { width: 1000, height: 500 };
  flushSync();
  expect(level.textContent).toBe("46%");
  // The buttons zoom in and out by 20%, and the level shows it whole again.
  (holder.querySelector(".zoom button:last-child") as HTMLButtonElement).click();
  flushSync();
  expect(level.textContent).toBe("55%");
  (level as HTMLButtonElement).click();
  flushSync();
  expect(shownBox(svg)).toEqual([0, 0, 1000, 500]);
  unmount(part);
  holder.remove();
});

test("on a canvas that zooms, a drag the handler does nothing with moves the view", async () => {
  const drops: Pointer[] = [];
  let calls = 0;
  const props = $state({
    drawing: [] as never[],
    units: { width: 1000, height: 500 },
    shown: String,
    zoom: true,
    ondrag: () => (calls++, Promise.resolve()),
    ondrop: (p: Pointer) => drops.push(p),
  });
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(CanvasView, { target: holder, props });
  flushSync();
  const svg = holder.querySelector("svg")!;
  svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 400, height: 200, right: 400, bottom: 200, x: 0, y: 0, toJSON: () => ({}) });
  const start = shownBox(svg)[0];
  svg.dispatchEvent(pointer("pointerdown", 200, 100));
  svg.dispatchEvent(pointer("pointermove", 160, 100));
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
  // The handler drew the same: the view moved 40 pixels, 100 units at 0.4 pixels a unit, to the right.
  expect(calls).toBe(1);
  expect(shownBox(svg)[0] - start).toBe(100);
  svg.dispatchEvent(pointer("pointermove", 120, 100));
  flushSync();
  expect(calls).toBe(1);
  expect(shownBox(svg)[0] - start).toBe(200);
  svg.dispatchEvent(pointer("pointerup", 120, 100));
  await new Promise((r) => setTimeout(r, 0));
  expect(drops).toEqual([]);
  // A handler that changes the drawing keeps the drag, and gets the drop.
  props.ondrag = () => ((props.drawing = [{ op: "clear", color: "panel" }] as never[]), Promise.resolve());
  flushSync();
  svg.dispatchEvent(pointer("pointerdown", 200, 100));
  svg.dispatchEvent(pointer("pointermove", 160, 100));
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
  expect(shownBox(svg)[0] - start).toBe(200);
  svg.dispatchEvent(pointer("pointerup", 160, 100));
  await new Promise((r) => setTimeout(r, 0));
  expect(drops.length).toBe(1);
  unmount(part);
  holder.remove();
});

test("a canvas with a grid shows its dots over the whole view, behind the shapes", () => {
  const clicks: Pointer[] = [];
  const drawing = [
    { op: "clear", color: "panel" },
    { op: "rect", x: 0, y: 0, width: 10, height: 10, color: "accent" },
  ] as never[];
  const { holder, svg, done } = canvas({ drawing, grid: 100, onpoint: (p: Pointer) => clicks.push(p) });
  const children = [...svg.children].map((c) => c.tagName.toLowerCase());
  // The pattern, the clear, the dots over the whole view, then the shapes.
  expect(children).toEqual(["defs", "rect", "rect", "rect"]);
  const dots = svg.children[2];
  expect(dots.getAttribute("fill")).toMatch(/^url\(#canvas-grid-/);
  expect([dots.getAttribute("width"), dots.getAttribute("height")]).toEqual(["1000", "500"]);
  // The pointer itself is not snapped: a click at 21, 31 is the unit under it.
  svg.dispatchEvent(pointer("pointerdown", 21, 31));
  svg.dispatchEvent(pointer("pointerup", 21, 31));
  expect(clicks).toEqual([{ x: 52, y: 77 }]);
  done();
});

test("on a canvas that zooms, the drawing's clear fills the whole view, past the canvas's edges", () => {
  const drawing = [{ op: "clear", color: "panel" }] as never[];
  const { svg, done } = canvas({ drawing, zoom: true, units: { width: 1000, height: 1000 } });
  // Whole: 0.2 pixels a unit shows 2000 x 1000 units, from -500 across.
  const clear = svg.querySelector("rect")!;
  expect([clear.getAttribute("x"), clear.getAttribute("y"), clear.getAttribute("width"), clear.getAttribute("height")]).toEqual(["-500", "0", "2000", "1000"]);
  done();
});

test("a clear without a color fills the canvas with its background, the theme's panel without one", () => {
  const drawing = [{ op: "clear", color: "" }] as never[];
  const papered = canvas({ drawing, background: "#f5f0e1" });
  expect(papered.svg.querySelector("rect")!.getAttribute("fill")).toBe("#f5f0e1");
  papered.done();
  const plain = canvas({ drawing, background: "" });
  expect(plain.svg.querySelector("rect")!.getAttribute("fill")).toBe("var(--panel)");
  plain.done();
});

test("a grid's dots have the size and color the canvas says, the same whatever the zoom", () => {
  const drawing = [{ op: "clear", color: "panel" }] as never[];
  // 0.4 pixels a unit: 3 pixels across is a radius of 3.75 units.
  const sized = canvas({ drawing, grid: 100, dotSize: 3, dotColor: "muted" });
  const dot = sized.svg.querySelector("pattern circle")!;
  expect([dot.getAttribute("r"), dot.getAttribute("fill")]).toEqual(["3.75", "var(--text-muted)"]);
  sized.done();
  const plain = canvas({ drawing, grid: 100 });
  const usual = plain.svg.querySelector("pattern circle")!;
  expect([usual.getAttribute("r"), usual.getAttribute("fill")]).toEqual(["2.5", "var(--border)"]);
  plain.done();
});
