// The diagram designer's lines bend: dragging a line adds a point where it
// was pressed and moves it, dragging a point moves it, and a double click
// on a point takes it away. Each gives the line's points as one edit.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import DesignCanvas from "../DesignCanvas.svelte";
import type { CanvasEdit, Link, Shape } from "../designCanvas";

function pointer(type: string, x: number, y: number): MouseEvent {
  const e = new MouseEvent(type, { clientX: x, clientY: y, button: 0, bubbles: true, cancelable: true });
  Object.defineProperty(e, "pointerId", { value: 1 });
  return e;
}

/** Two boxes side by side with a line between them; the canvas at 100%, at the page's corner. */
function designer(via: { x: number; y: number }[]) {
  const shapes: Shape[] = [
    { name: "A", x: 0, y: 0, rows: [["text", "name"]] },
    { name: "B", x: 400, y: 0, rows: [["text", "name"]] },
  ];
  const links: Link[] = [{ left: "A", right: "B", leftEnd: { kind: "count", count: "exactlyOne" }, rightEnd: { kind: "count", count: "zeroOrMore" }, dashed: false, label: null, via }];
  const edits: CanvasEdit[] = [];
  const holder = document.createElement("div");
  document.body.append(holder);
  const part = mount(DesignCanvas, { target: holder, props: { shapes, links, onedit: (edit: CanvasEdit) => (edits.push(edit), undefined) } });
  flushSync();
  const svg = holder.querySelector("svg")!;
  svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 800, height: 600, right: 800, bottom: 600, x: 0, y: 0, toJSON: () => ({}) });
  return { holder, edits, done: () => (unmount(part), holder.remove()) };
}

test("dragging a straight line adds a bend point where it was pressed, and moves it", () => {
  const { holder, edits, done } = designer([]);
  const wire = holder.querySelector(".link polyline.hit")!;
  const [ax] = wire.getAttribute("points")!.split(" ")[0].split(",").map(Number);
  const [bx, by] = wire.getAttribute("points")!.split(" ")[1].split(",").map(Number);
  const mx = (ax + bx) / 2;
  wire.dispatchEvent(pointer("pointerdown", mx, by));
  window.dispatchEvent(pointer("pointermove", mx, by + 60));
  window.dispatchEvent(pointer("pointerup", mx, by + 60));
  flushSync();
  expect(edits).toEqual([{ op: "bend", index: 0, via: [{ x: mx, y: by + 60 }] }]);
  done();
});

test("dragging a bend point moves it, and a double click takes it away", () => {
  const { holder, edits, done } = designer([{ x: 300, y: 200 }]);
  // Picked, the line shows its point.
  holder.querySelector(".link polyline.hit")!.dispatchEvent(pointer("pointerdown", 300, 200));
  window.dispatchEvent(pointer("pointerup", 300, 200));
  flushSync();
  const point = holder.querySelector("circle.bend")!;
  expect([point.getAttribute("cx"), point.getAttribute("cy")]).toEqual(["300", "200"]);
  holder.querySelector(".link polyline.hit")!.dispatchEvent(pointer("pointerdown", 301, 199));
  window.dispatchEvent(pointer("pointermove", 341, 239));
  window.dispatchEvent(pointer("pointerup", 341, 239));
  flushSync();
  expect(edits.at(-1)).toEqual({ op: "bend", index: 0, via: [{ x: 340, y: 240 }] });
  holder.querySelector("circle.bend")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  expect(edits.at(-1)).toEqual({ op: "bend", index: 0, via: [] });
  done();
});

test("Arrange lays every box out again in layers, in one edit", () => {
  const { holder, edits, done } = designer([]);
  const arrange = [...holder.querySelectorAll(".zoom button")].find((b) => b.textContent === "Arrange") as HTMLButtonElement;
  arrange.click();
  flushSync();
  const moves = (edits.at(-1) as Extract<CanvasEdit, { op: "moveAll" }>).moves;
  const at = Object.fromEntries(moves.map((m) => [m.name, m]));
  // The line's one end is B, which A refers to: B stands above A.
  expect(at.B.y).toBeLessThan(at.A.y);
  done();
});
