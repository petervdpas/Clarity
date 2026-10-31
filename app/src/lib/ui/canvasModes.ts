// What a canvas with a mode shows and how a gesture reads (DESIGN 5.31).
// The runtime decides what an action does (it snaps, keeps the SVG and
// chooses what was touched); the screen finds what the pointer is on, draws
// the gesture while it goes, and sends it.

import type { Shape } from "../drawing";

type Point = { x: number; y: number };

/** An object of a canvas in mode objects: its record's id, its box, and the shapes that are it (from, to). */
export type CanvasObject = { record: unknown; x: number; y: number; width: number; height: number; from: number; to: number };

const n = (value: unknown) => (typeof value === "number" ? value : Number(value) || 0);

/** The objects of a drawing: each `object` box with the shapes drawn after it until the next object. */
export function objectsOf(drawing: Shape[]): CanvasObject[] {
  const objects: CanvasObject[] = [];
  drawing.forEach((shape, i) => {
    if (shape.op !== "object") return;
    const last = objects.at(-1);
    if (last) last.to = i;
    objects.push({ record: shape.record, x: n(shape.x), y: n(shape.y), width: n(shape.width), height: n(shape.height), from: i, to: drawing.length });
  });
  return objects;
}

/** The topmost object whose box holds the point; null when none does. */
export function objectAt(objects: CanvasObject[], p: Point): CanvasObject | null {
  for (let i = objects.length - 1; i >= 0; i--) {
    const o = objects[i];
    if (p.x >= o.x && p.x <= o.x + o.width && p.y >= o.y && p.y <= o.y + o.height) return o;
  }
  return null;
}

/** The shape a drag of a tool makes through its points, in the ink and brush; null when it is too small to be one. */
export function shapeOfDrag(tool: string, points: Point[], ink: string, brush: number): Shape | null {
  const from = points[0];
  const to = points.at(-1);
  if (!from || !to) return null;
  const r = (v: number) => Math.round(v);
  const width = Math.abs(to.x - from.x);
  const height = Math.abs(to.y - from.y);
  switch (tool) {
    case "line":
      return width || height ? { op: "line", x1: r(from.x), y1: r(from.y), x2: r(to.x), y2: r(to.y), color: ink, width: brush } : null;
    case "rect":
      return width && height ? { op: "rect", x: r(Math.min(from.x, to.x)), y: r(Math.min(from.y, to.y)), width: r(width), height: r(height), color: ink } : null;
    case "ellipse":
      return width && height ? { op: "ellipse", x: r((from.x + to.x) / 2), y: r((from.y + to.y) / 2), rx: r(width / 2), ry: r(height / 2), color: ink } : null;
    case "path":
      return points.length > 1 ? { op: "path", points: points.flatMap((p) => [r(p.x), r(p.y)]), color: ink, width: brush } : null;
  }
  return null;
}

/** The first place of a drawn element, which a move puts where the person dropped it; null for one the runtime does not move. */
export function anchorOf(tag: string, attrs: Record<string, string>): Point | null {
  switch (tag) {
    case "rect":
    case "text":
      return { x: n(attrs.x), y: n(attrs.y) };
    case "ellipse":
      return { x: n(attrs.cx), y: n(attrs.cy) };
    case "line":
      return { x: n(attrs.x1), y: n(attrs.y1) };
    case "polyline": {
      const [x, y] = (attrs.points ?? "").trim().split(/[\s,]+/).map(Number);
      return { x: x || 0, y: y || 0 };
    }
  }
  return null;
}

/** An element of a drawing as the canvas shows it. */
export type DrawnElement = { tag: string; attrs: Record<string, string>; text: string };

/** The shapes a drawing shows, and the attributes they may have: nothing that runs or links. */
const SHOWN: Record<string, string[]> = {
  rect: ["x", "y", "width", "height", "rx", "ry"],
  line: ["x1", "y1", "x2", "y2"],
  ellipse: ["cx", "cy", "rx", "ry"],
  circle: ["cx", "cy", "r"],
  polyline: ["points"],
  polygon: ["points"],
  path: ["d"],
  text: ["x", "y", "font-size", "text-anchor"],
};
const PAINT = ["fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin", "opacity"];

/**
 * A vector drawing's SVG text as the canvas shows it: its units (its
 * viewBox) and each element on its own, in order, so the place of an element
 * is the place of its shape. Elements and attributes outside the shapes a
 * drawing has (a script, an event, a link) show nothing; a link's content too.
 */
export function cleanSvg(text: string, units: [number, number] = [1000, 1000]): { units: [number, number]; shapes: DrawnElement[] } {
  if (!text.trim()) return { units, shapes: [] };
  const doc = new DOMParser().parseFromString(text, "image/svg+xml");
  const root = doc.documentElement;
  if (root.nodeName !== "svg") return { units, shapes: [] };
  const box = (root.getAttribute("viewBox") ?? "").trim().split(/[\s,]+/).map(Number);
  const own: [number, number] = box.length === 4 && box[2] > 0 && box[3] > 0 ? [box[2], box[3]] : units;
  const shapes: DrawnElement[] = [];
  for (const element of Array.from(root.children)) {
    const tag = element.nodeName;
    const allowed = SHOWN[tag];
    // Left out, it still counts: the place of each element is the place of its shape.
    if (!allowed) {
      shapes.push({ tag: "g", attrs: {}, text: "" });
      continue;
    }
    const attrs: Record<string, string> = {};
    for (const name of [...allowed, ...PAINT]) {
      const value = element.getAttribute(name);
      if (value !== null && !/url\s*\(|javascript:/i.test(value)) attrs[name] = value;
    }
    shapes.push({ tag, attrs, text: tag === "text" ? (element.textContent ?? "") : "" });
  }
  return { units: own, shapes };
}
