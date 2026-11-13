// What the properties panel works out for a part's lists: columns in their
// written order, link and drop handlers, a canvas's tools and objects.

import type { CanvasMode } from "@clarity/ui/generated/CanvasMode.ts";
import type { CanvasObjects } from "@clarity/ui/generated/CanvasObjects.ts";
import type { ColumnLink } from "@clarity/ui/generated/ColumnLink.ts";
import type { DropFrom } from "@clarity/ui/generated/DropFrom.ts";
import type { WidgetKind } from "@clarity/ui/generated/WidgetKind.ts";

/** The tools of each canvas mode, in the order its toolbar shows them (clarity_spec::model::CanvasMode::tools). */
export const CANVAS_MODE_TOOLS: Record<CanvasMode, readonly string[]> = {
  objects: ["select", "connect"],
  paint: ["pen", "pencil", "spray", "eraser", "fill", "picker", "select", "ellipseSelect", "lasso", "line", "rect", "ellipse"],
  vector: ["select", "line", "rect", "ellipse", "path", "text"],
};

/** What a toolbox's tool may do built in, `does pencil` (clarity_spec::model::TOOL_ACTIONS). */
export const TOOL_ACTIONS: readonly string[] = [
  "pen", "pencil", "spray", "eraser", "fill", "picker", "select", "ellipseSelect", "lasso", "line", "rect", "ellipse", "path", "text", "connect",
  "rotate", "flipAcross", "flipDown", "crop", "import", "export", "zoomIn", "zoomOut", "adjust",
];

/** The parts that take rows dragged onto them, `on drop from` (the parser's list). */
export const DROP_KINDS: readonly WidgetKind[] = ["list", "table", "tree", "treeTable", "memo"];

/** The parts rows are dragged from. */
const ROW_KINDS: readonly WidgetKind[] = ["list", "table", "tree", "treeTable"];

/** `name` ticked on goes last; ticked off it goes. The rest keep the order as written. */
export function toggled(list: string[], name: string, on: boolean): string[] {
  if (on) return list.includes(name) ? list : [...list, name];
  return list.filter((n) => n !== name);
}

/** `name` moved one place up (-1) or down (1); at an end it stays. */
export function moved(list: string[], name: string, by: -1 | 1): string[] {
  const from = list.indexOf(name);
  const to = from + by;
  if (from < 0 || to < 0 || to >= list.length) return list;
  const next = [...list];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
}

/** The fields as the panel lists them: the chosen ones in their order, then the others in the entity's. */
export function shownOrder(all: string[], chosen: string[]): string[] {
  return [...chosen.filter((n) => all.includes(n)), ...all.filter((n) => !chosen.includes(n))];
}

/** A column's link handler set, or taken away with `null`. */
export function withLink(links: ColumnLink[], column: string, handler: string | null): ColumnLink[] {
  if (handler === null) return links.filter((l) => l.column !== column);
  return links.some((l) => l.column === column) ? links.map((l) => (l.column === column ? { column, handler } : l)) : [...links, { column, handler }];
}

/** The handler of the rows taken from `from` set, or the drop taken away with `null`. */
export function withDrop(drops: DropFrom[], from: string, handler: string | null): DropFrom[] {
  if (handler === null) return drops.filter((d) => d.from !== from);
  return drops.some((d) => d.from === from) ? drops.map((d) => (d.from === from ? { from, handler } : d)) : [...drops, { from, handler }];
}

/** The window's named lists, tables and trees, besides `self`, that rows can be dragged from. */
export function dropSources(parts: readonly { name: string | null; kind: WidgetKind }[], self: string | null): string[] {
  return parts.filter((p) => p.name !== null && p.name !== self && ROW_KINDS.includes(p.kind)).map((p) => p.name as string);
}

/** A canvas's tools after one is ticked on or off; none written means all of the mode's, and the last one stays. */
export function toolsToggled(tools: string[], mode: CanvasMode, tool: string, on: boolean): string[] {
  const all = CANVAS_MODE_TOOLS[mode];
  const current = tools.length > 0 ? tools : [...all];
  const next = toggled(current, tool, on);
  if (next.length === 0) return current;
  return all.every((t) => next.includes(t)) ? [] : next;
}

/** A canvas's objects on `list`, placed by its fields `x` and `y` when it has them, else by its first integer fields. */
export function objectsFor(list: string, integers: string[]): CanvasObjects | null {
  if (integers.length === 0) return null;
  const x = integers.includes("x") ? "x" : integers[0];
  const y = integers.includes("y") ? "y" : (integers.find((n) => n !== x) ?? x);
  return { list, x, y };
}
