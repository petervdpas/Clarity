// The designer's layer over the window Clarity.UI draws (DESIGN 8.1): a
// box for each part and each grid, placed where the library reports them
// (`Layout`, its `layout` step), each known by its key (its file and name,
// `design/Main.cxaml#Main/saveButton`, see `DesignKey`). Pure, so the
// placing is tested without a browser.

import { ROW_HEIGHT } from "@clarity/ui/screen/grid.ts";
import type { Command } from "@clarity/ui/generated/Command.ts";
import type { Frame } from "@clarity/ui/generated/Frame.ts";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { SlideSide } from "@clarity/ui/generated/SlideSide.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import type { DesignKey } from "@clarity/ui/screen/pages.ts";
import { sidebarWidth } from "@clarity/ui/frame/sidebars.ts";
import type { Layout } from "./clarityWindow";

export type Box = { x: number; y: number; width: number; height: number };

/** The key of a thing of the window shown (`ShownPage.keyOf`). */
export type KeyOf = (thing: { span: { start: number } }) => DesignKey | null;

/** A page of a container: its key, and the header that shows it, where it has one. */
export type LayerPage = { item: DesignKey; title: string | null; box: Box | null };

/**
 * A part of the design as the layer shows it; a frame's button is a
 * `command`, the window's own (`own`) or its app's. A container's pages and
 * a table's columns come with it.
 */
export type LayerPart = { item: DesignKey; name: string | null; kind: string; box: Box; own: boolean; pages: LayerPage[]; columns: { field: string; box: Box }[] };

/** A grid of the design: what holds it (the window, a container, a page), its columns and rows, and the containers it lies in. */
export type LayerGrid = { item: DesignKey; columns: number; gap: number; box: Box; rows: number[]; inside: DesignKey[] };

function boxOf(at: [number, number, number, number]): Box {
  return { x: at[0], y: at[1], width: at[2], height: at[3] };
}

function itemOf(item: string | null | undefined): DesignKey | null {
  return item === null || item === undefined || item === "" ? null : item;
}

/** The keys of every container inside `widget`, its own grid and pages too. */
export function heldBy(widget: Widget, keyOf: KeyOf): DesignKey[] {
  const out = [keyOf(widget)];
  for (const page of widget.pages) out.push(keyOf(page));
  for (const child of [...widget.children, ...widget.pages.flatMap((p) => p.children)]) out.push(...heldBy(child, keyOf));
  return out.filter((k): k is DesignKey => k !== null);
}

/** Every part of the window, by its key. */
function partsOf(page: Page, keyOf: KeyOf): Map<DesignKey, Widget> {
  const out = new Map<DesignKey, Widget>();
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      const key = keyOf(w);
      if (key !== null) out.set(key, w);
      visit(w.children);
      for (const inner of w.pages) visit(inner.children);
    }
  };
  visit(page.widgets);
  for (const sidebar of page.frame.sidebars) visit(sidebar.widgets);
  if (page.frame.footer) visit(page.frame.footer.widgets);
  return out;
}

/** The keys of a frame's buttons and menu items, those of submenus too. */
export function commandsOf(frame: Frame | null | undefined, keyOf: KeyOf): DesignKey[] {
  if (!frame) return [];
  const out: DesignKey[] = [];
  const visit = (items: Command[]) => {
    for (const c of items) {
      const key = keyOf(c);
      if (key !== null) out.push(key);
      visit(c.items);
    }
  };
  if (frame.menu) visit(frame.menu.items);
  if (frame.toolbar) visit(frame.toolbar.items);
  for (const tab of frame.ribbon?.tabs ?? []) for (const group of tab.groups) visit(group.items);
  return out;
}

/**
 * The layer's parts and grids from what Clarity.UI reports: only what the
 * design has (a key it knows), each part once (the first drawn), and each
 * grid with the parts it lies inside, so a part is never dropped into itself.
 * A frame's buttons are known by the window's own frame, or else by its
 * app's (`appCommands`, their keys), whose buttons are designed in the app.
 * The grid of a line of a window's content is its view's (`views`, line to
 * view), as the view's parts lie in the view.
 */
export function layerOf(layout: Layout, page: Page, keyOf: KeyOf, appCommands: Set<DesignKey> = new Set(), views: Map<DesignKey, DesignKey> = new Map()): { parts: LayerPart[]; grids: LayerGrid[] } {
  const known = partsOf(page, keyOf);
  const own = new Set(commandsOf(page.frame, keyOf));
  const seen = new Set<DesignKey>();
  const parts: LayerPart[] = [];
  for (const laid of layout.parts) {
    const item = itemOf(laid.item);
    const command = laid.kind === "command";
    if (item === null || seen.has(item)) continue;
    if (command ? !own.has(item) && !appCommands.has(item) : !known.has(item)) continue;
    seen.add(item);
    // A line of a window's content has no title: it is named by the view it shows.
    const lines = known.get(item)?.pages ?? [];
    const pages = (laid.pages ?? []).flatMap((p) => {
      const at = itemOf(p.item);
      const view = lines.find((l) => keyOf(l) === at)?.view ?? null;
      return at === null ? [] : [{ item: at, title: p.title ?? view, box: p.at ? boxOf(p.at) : null }];
    });
    const columns = (laid.columns ?? []).map((c) => ({ field: c.field, box: boxOf(c.at) }));
    parts.push({ item, name: laid.name, kind: laid.kind, box: boxOf(laid.at), own: command ? own.has(item) : true, pages, columns });
  }
  const within = new Map<DesignKey, DesignKey[]>();
  for (const [key, widget] of known) {
    for (const held of heldBy(widget, keyOf)) within.set(held, [...(within.get(held) ?? []), key]);
  }
  const grids: LayerGrid[] = [];
  for (const laid of layout.grids) {
    const item = itemOf(laid.item);
    if (item === null) continue;
    const named = views.get(item) ?? item;
    grids.push({ item: named, columns: laid.columns, gap: laid.gap, box: boxOf(laid.at), rows: laid.rows, inside: within.get(item) ?? within.get(named) ?? [] });
  }
  return { parts, grids };
}

/** The cells of a grid, each column by each row, in its own pixels from its top left; with `row`, rows of that height go on below the last laid out, to the grid's bottom. */
export function cellsOf(grid: LayerGrid, row?: number): Box[] {
  const column = (grid.box.width - grid.gap * (grid.columns - 1)) / grid.columns;
  const out: Box[] = [];
  let y = 0;
  const across = (height: number) => {
    for (let c = 0; c < grid.columns; c++) out.push({ x: c * (column + grid.gap), y, width: column, height });
    y += height + grid.gap;
  };
  for (const height of grid.rows) across(height);
  // The empty rows to the bottom: where a part may still be dropped.
  if (row && row > 0) {
    while (y + row <= grid.box.height + 0.5) across(row);
  }
  return out;
}

/** Where a part dropped at `col`, `row`, `width` columns by `height` rows would be, in the grid's own pixels; rows past the last are `row` high, the project's row height. */
export function placeIn(grid: LayerGrid, at: { col: number; row: number; width: number; height: number }, row = ROW_HEIGHT): Box {
  const column = (grid.box.width - grid.gap * (grid.columns - 1)) / grid.columns;
  const rowHeight = (r: number) => grid.rows[r - 1] ?? row;
  let top = 0;
  for (let r = 1; r < at.row; r++) top += rowHeight(r) + grid.gap;
  let height = 0;
  for (let r = at.row; r < at.row + at.height; r++) height += rowHeight(r) + (r > at.row ? grid.gap : 0);
  return { x: (at.col - 1) * (column + grid.gap), y: top, width: at.width * column + (at.width - 1) * grid.gap, height };
}

/** Where an arrow key moves a part in a grid of `columns`: one cell that way; null for another key, or at the grid's edge. */
export function nudged(at: { col: number; row: number; width: number }, columns: number, key: string): { col: number; row: number } | null {
  const step: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
  const by = step[key];
  if (!by) return null;
  const col = at.col + by[0];
  const row = at.row + by[1];
  if (col < 1 || row < 1 || col + at.width - 1 > columns) return null;
  return { col, row };
}

/** How Shift and an arrow key resize a part in a grid of `columns`: one cell wider (right), narrower (left), taller (down) or shorter (up); null for another key, below one cell, or past the grid's edge. */
export function grown(at: { col: number; width: number; height: number }, columns: number, key: string): { width: number; height: number } | null {
  const step: Record<string, [number, number]> = { ArrowRight: [1, 0], ArrowLeft: [-1, 0], ArrowDown: [0, 1], ArrowUp: [0, -1] };
  const by = step[key];
  if (!by) return null;
  const width = at.width + by[0];
  const height = at.height + by[1];
  if (width < 1 || height < 1 || at.col + width - 1 > columns) return null;
  return { width, height };
}

/** How far a resize handle moves for one row: a row of the design's height and the gap below it, however high the grid is drawn; without a row height, the grid's height shared by its rows. */
export function rowStep(rowHeight: number, gap: number, height: number, rows: number): number {
  return rowHeight > 0 ? rowHeight + gap : height / Math.max(1, rows);
}

/** The cells a resize handle moved: the pointer's move from `from` to `to`, rounded to whole columns and rows. */
export function resizedBy(from: { x: number; y: number }, to: { x: number; y: number }, column: number, row: number): { columns: number; rows: number } {
  return { columns: Math.round((to.x - from.x) / column), rows: Math.round((to.y - from.y) / row) };
}

/** A part's size after its handle moved `by` cells: at least one cell, and not past its grid's last column. */
export function resizedTo(at: { col: number; width: number; height: number }, by: { columns: number; rows: number }, columns: number): { width: number; height: number } {
  return { width: Math.min(columns - at.col + 1, Math.max(1, at.width + by.columns)), height: Math.max(1, at.height + by.rows) };
}

/**
 * Where a splitter's divider lies, between its two panes' grids as
 * Clarity.UI laid them out: across it one above the other (`stacked`), down
 * it side by side, 6 pixels thick to take the pointer. None until both
 * panes are laid out.
 */
export function dividerOf(part: LayerPart, grids: LayerGrid[], stacked: boolean): Box | null {
  const [first, second] = part.pages.map((p) => grids.find((g) => g.item === p.item)?.box);
  if (!first || !second) return null;
  if (stacked) {
    const y = Math.round((first.y + first.height + second.y) / 2);
    return { x: part.box.x, y: y - 3, width: part.box.width, height: 6 };
  }
  const x = Math.round((first.x + first.width + second.x) / 2);
  return { x: x - 3, y: part.box.y, width: 6, height: part.box.height };
}

/**
 * The size a splitter's first pane starts with when its divider is let go
 * at `at` (down it stacked, across it side by side, in the layer's pixels):
 * its share of the splitter in the splitter's `length` rows or columns,
 * from 1 to one less, as `rows` and `span` on the pane say.
 */
export function paneSizeAt(part: LayerPart, at: number, length: number, stacked: boolean): number {
  const start = stacked ? part.box.y : part.box.x;
  const whole = stacked ? part.box.height : part.box.width;
  const share = whole > 0 ? (at - start) / whole : 0.5;
  return Math.min(length - 1, Math.max(1, Math.round(share * length)));
}

/**
 * Where a slide-out's inner edge lies on the canvas: on the side away from
 * the window's edge it comes in from (`side`, the right without one), 6
 * pixels thick to take the pointer.
 */
export function slideEdgeOf(part: LayerPart, side: SlideSide | null): Box {
  const b = part.box;
  switch (side ?? "right") {
    case "left":
      return { x: b.x + b.width - 3, y: b.y, width: 6, height: b.height };
    case "top":
      return { x: b.x, y: b.y + b.height - 3, width: b.width, height: 6 };
    case "bottom":
      return { x: b.x, y: b.y - 3, width: b.width, height: 6 };
    default:
      return { x: b.x - 3, y: b.y, width: 6, height: b.height };
  }
}

/**
 * The size a slide-out takes when its inner edge is let go at `at` (across
 * it at the left or right, down it at the top or bottom, in the layer's
 * pixels): from its outer edge to there, in whole pixels, within its `min`
 * and `max` and the widths a sidebar may have, as its width or height says.
 */
export function slideSizeAt(part: LayerPart, side: SlideSide | null, at: number, min: number | null, max: number | null): number {
  const b = part.box;
  const size = side === "left" ? at - b.x : side === "top" ? at - b.y : side === "bottom" ? b.y + b.height - at : b.x + b.width - at;
  return sidebarWidth(size, min, max);
}
