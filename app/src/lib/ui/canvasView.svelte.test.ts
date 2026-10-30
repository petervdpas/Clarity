// A canvas the person zooms and drags on (conformance/drawing.json, views):
// a drag runs its handler one call at a time with the latest place and where
// it was pressed, the drop after the drags, and a press that does not move
// is a click. A zoomed view stays when the drawing changes.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import type { Pointer } from "../live";
import CanvasView from "./CanvasView.svelte";

// The simulated page measures no sizes; the canvas watches its part's size all the same.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

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

test("the wheel zooms and scrolls a canvas that zooms, and a redraw keeps the view", () => {
  const props = $state({ drawing: [] as never[], units: { width: 1000, height: 500 }, shown: String, zoom: true });
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(CanvasView, { target: holder, props });
  flushSync();
  const svg = holder.querySelector("svg")!;
  svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 400, height: 200, right: 400, bottom: 200, x: 0, y: 0, toJSON: () => ({}) });
  expect(svg.getAttribute("viewBox")).toBe("0 0 1000 500");
  // Ctrl and the wheel zoom in around the pointer, here the middle.
  svg.dispatchEvent(new WheelEvent("wheel", { deltaY: -300 * Math.log(2), ctrlKey: true, clientX: 200, clientY: 100, cancelable: true }));
  flushSync();
  const [x, y, w, h] = svg.getAttribute("viewBox")!.split(" ").map(Number);
  expect([x, y, w, h].map((v) => Math.round(v))).toEqual([250, 125, 500, 250]);
  expect(holder.querySelector(".bar.across span")).not.toBeNull();
  // The wheel alone scrolls: 40 pixels down is 50 units, at 0.8 pixels a unit.
  svg.dispatchEvent(new WheelEvent("wheel", { deltaY: 40, clientX: 200, clientY: 100, cancelable: true }));
  flushSync();
  expect(Math.round(Number(svg.getAttribute("viewBox")!.split(" ")[1]))).toBe(175);
  // A new drawing, and new units of the same size, keep the view.
  props.drawing = [];
  props.units = { width: 1000, height: 500 };
  flushSync();
  expect(Math.round(Number(svg.getAttribute("viewBox")!.split(" ")[2]))).toBe(500);
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
  // Zoomed in twice: 500 x 250 units in view, 0.8 pixels a unit.
  svg.dispatchEvent(new WheelEvent("wheel", { deltaY: -300 * Math.log(2), ctrlKey: true, clientX: 200, clientY: 100, cancelable: true }));
  flushSync();
  svg.dispatchEvent(pointer("pointerdown", 200, 100));
  svg.dispatchEvent(pointer("pointermove", 160, 100));
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
  // The handler drew the same: the view moved 40 pixels, 50 units, to the right.
  expect(calls).toBe(1);
  expect(Math.round(Number(svg.getAttribute("viewBox")!.split(" ")[0]))).toBe(300);
  svg.dispatchEvent(pointer("pointermove", 120, 100));
  flushSync();
  expect(calls).toBe(1);
  expect(Math.round(Number(svg.getAttribute("viewBox")!.split(" ")[0]))).toBe(350);
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
  expect(Math.round(Number(svg.getAttribute("viewBox")!.split(" ")[0]))).toBe(350);
  svg.dispatchEvent(pointer("pointerup", 160, 100));
  await new Promise((r) => setTimeout(r, 0));
  expect(drops.length).toBe(1);
  unmount(part);
  holder.remove();
});

test("a canvas with a grid shows it behind the shapes, and snaps the places it gives", () => {
  const clicks: Pointer[] = [];
  const drawing = [
    { op: "clear", color: "panel" },
    { op: "rect", x: 0, y: 0, width: 10, height: 10, color: "accent" },
  ] as never[];
  const { holder, svg, done } = canvas({ drawing, grid: 100, snap: true, onpoint: (p: Pointer) => clicks.push(p) });
  const children = [...svg.children].map((c) => c.tagName.toLowerCase());
  // The clear, then the grid, then the shapes.
  expect(children).toEqual(["rect", "g", "rect"]);
  expect(holder.querySelectorAll(".grid line").length).toBeGreaterThan(0);
  // 2.5 units a pixel: a click at 20, 30 is 50, 75, which snaps to 100, 100.
  svg.dispatchEvent(pointer("pointerdown", 20, 30));
  svg.dispatchEvent(pointer("pointerup", 20, 30));
  expect(clicks).toEqual([{ x: 100, y: 100 }]);
  done();
});
