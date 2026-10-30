// A canvas drawn by code: the shapes its draw handler sends, as the web
// renderer draws them in SVG, in the canvas's own units
// (conformance/drawing.json). ui/CanvasView.svelte puts them on screen.

import type { ShownText } from "./format";

/** The units a canvas draws in without `units W x H` (clarity_spec::widgets::DEFAULT_UNITS). */
export const DEFAULT_UNITS = 100;

/** A shape as the backend sends it: `{ op: "line", x1, y1, ... }`. */
export type Shape = { op: string; [value: string]: unknown };

/** The CSS variable of each color role, as the theme CSS names them
 *  (clarity_gen::web COLOR_VARIABLES). */
const ROLES: Record<string, string> = {
  background: "--bg",
  panel: "--panel",
  raised: "--panel-alt",
  text: "--text",
  muted: "--text-muted",
  border: "--border",
  strong_border: "--border-strong",
  accent: "--accent",
  accent_soft: "--accent-soft",
  on_accent: "--on-accent",
  selection: "--selection",
  focus: "--focus",
  input: "--input",
  frame: "--frame",
  frame_text: "--frame-text",
  error: "--error",
  error_soft: "--error-soft",
  warning: "--warning",
  warning_soft: "--warning-soft",
  success: "--success",
  success_soft: "--success-soft",
};

/** A color as CSS: written out as is, a role or a color of the project's
 *  theme by its variable; a name the theme does not have is its text color. */
export function colorOf(color: unknown): string {
  const name = typeof color === "string" ? color : "text";
  if (name.startsWith("#")) return name;
  const role = ROLES[name];
  return role ? `var(${role})` : `var(--color-${name.replaceAll("_", "-")}, var(--text))`;
}

const n = (value: unknown) => (typeof value === "number" ? value : Number(value) || 0);

/** A shape as an SVG element: its tag, its attributes, and its text. */
export function svgOf(
  shape: Shape,
  units: { width: number; height: number },
  shown: (text: ShownText) => string,
  picture: (name: string) => string | null | undefined = () => null,
): { tag: string; attrs: Record<string, string | number>; text?: string } {
  const color = colorOf(shape.color);
  switch (shape.op) {
    case "clear":
      return { tag: "rect", attrs: { x: 0, y: 0, width: units.width, height: units.height, fill: color } };
    case "line":
      return {
        tag: "line",
        attrs: { x1: n(shape.x1), y1: n(shape.y1), x2: n(shape.x2), y2: n(shape.y2), stroke: color, "stroke-width": n(shape.width ?? 1), "stroke-linecap": "round" },
      };
    case "rect":
      return { tag: "rect", attrs: { x: n(shape.x), y: n(shape.y), width: n(shape.width), height: n(shape.height), fill: color } };
    case "circle":
      return { tag: "circle", attrs: { cx: n(shape.x), cy: n(shape.y), r: n(shape.r), fill: color } };
    case "ring":
      return { tag: "circle", attrs: { cx: n(shape.x), cy: n(shape.y), r: n(shape.r), fill: "none", stroke: color, "stroke-width": n(shape.width ?? 1) } };
    case "text":
      return {
        tag: "text",
        attrs: { x: n(shape.x), y: n(shape.y), fill: color, "font-size": n(shape.size ?? 10), "text-anchor": "middle", "dominant-baseline": "central" },
        text: shown(shape.text as ShownText),
      };
    // A picture of the resources folder, stretched to fill its box; one that is not there draws nothing.
    case "image": {
      const href = picture(String(shape.picture ?? ""));
      if (!href) return { tag: "g", attrs: {} };
      return { tag: "image", attrs: { x: n(shape.x), y: n(shape.y), width: n(shape.width), height: n(shape.height), href, preserveAspectRatio: "none" } };
    }
    default:
      return { tag: "g", attrs: {} };
  }
}

/**
 * Where in a canvas's units a click is, from where in the part it is (in
 * pixels), the drawing fitted whole and centred as the canvas draws it;
 * null for a click beside the drawing (conformance/drawing.json, clicks).
 */
export function pointOf(
  units: { width: number; height: number },
  part: { width: number; height: number },
  at: { x: number; y: number },
): { x: number; y: number } | null {
  const scale = Math.min(part.width / units.width, part.height / units.height);
  if (!(scale > 0)) return null;
  const x = (at.x - (part.width - units.width * scale) / 2) / scale;
  const y = (at.y - (part.height - units.height * scale) / 2) / scale;
  if (x < 0 || y < 0 || x >= units.width || y >= units.height) return null;
  return { x: Math.floor(x), y: Math.floor(y) };
}

/** The rectangle of a canvas's units a part shows: `[x, y, width, height]`. */
export type View = [number, number, number, number];

/** A canvas zooms in to a sixteenth of its width at most. */
const CLOSEST = 16;

/** The view kept inside the canvas: never larger, and moved back within it. */
function within(units: { width: number; height: number }, x: number, y: number, width: number, height: number): View {
  return [Math.min(Math.max(x, 0), units.width - width), Math.min(Math.max(y, 0), units.height - height), width, height];
}

/**
 * The view after zooming by `factor` (above 1 closer) around `at`, in
 * units, which stays where it is (conformance/drawing.json, views).
 */
export function zoomed(units: { width: number; height: number }, view: View, factor: number, at: { x: number; y: number }): View {
  const width = Math.min(Math.max(view[2] / factor, units.width / CLOSEST), units.width);
  const height = (width * units.height) / units.width;
  const kept = width / view[2];
  return within(units, at.x - (at.x - view[0]) * kept, at.y - (at.y - view[1]) * kept, width, height);
}

/** The view moved by `by` units, kept inside the canvas. */
export function scrolled(units: { width: number; height: number }, view: View, by: { x: number; y: number }): View {
  return within(units, view[0] + by.x, view[1] + by.y, view[2], view[3]);
}

/**
 * Where in a canvas's units a pointer in the part is, through the view it
 * shows, fitted and centred; null beside the drawing. With the whole canvas
 * as its view this is pointOf.
 */
export function pointIn(view: View, part: { width: number; height: number }, at: { x: number; y: number }): { x: number; y: number } | null {
  const scale = Math.min(part.width / view[2], part.height / view[3]);
  if (!(scale > 0)) return null;
  const x = (at.x - (part.width - view[2] * scale) / 2) / scale;
  const y = (at.y - (part.height - view[3] * scale) / 2) / scale;
  if (x < 0 || y < 0 || x >= view[2] || y >= view[3]) return null;
  return { x: Math.floor(view[0] + x), y: Math.floor(view[1] + y) };
}

/**
 * Where in a canvas's units a pointer that is down is, as pointOf finds it,
 * but beside the drawing at its nearest edge; null when the part shows
 * nothing (conformance/screen-engines.json, near).
 */
export function pointNear(
  units: { width: number; height: number },
  part: { width: number; height: number },
  at: { x: number; y: number },
): { x: number; y: number } | null {
  const scale = Math.min(part.width / units.width, part.height / units.height);
  if (!(scale > 0) || !Number.isFinite(scale)) return null;
  const x = (at.x - (part.width - units.width * scale) / 2) / scale;
  const y = (at.y - (part.height - units.height * scale) / 2) / scale;
  const edge = (v: number, size: number) => Math.min(Math.max(Math.floor(v), 0), Math.ceil(size) - 1);
  return { x: edge(x, units.width), y: edge(y, units.height) };
}
