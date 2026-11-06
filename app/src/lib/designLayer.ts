// The designer's layer over the window Clarity.UI draws (DESIGN 8.1): a
// box for each part and each grid, placed where the library reports them
// (`Layout`, its `layout` step), each known by where it is written in the
// project (its item, the offset the designer's edits name). Pure, so the
// placing is tested without a browser.

import { ROW_HEIGHT } from "@clarity/ui/screen/grid.ts";
import type { Command } from "@clarity/ui/generated/Command.ts";
import type { Frame } from "@clarity/ui/generated/Frame.ts";
import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import type { Layout } from "./clarityWindow";

export type Box = { x: number; y: number; width: number; height: number };

/** A page of a container: the item of the page, and the header that shows it, where it has one. */
export type LayerPage = { item: number; title: string | null; box: Box | null };

/**
 * A part of the design as the layer shows it; a frame's button is a
 * `command`, the window's own (`own`) or its app's. A container's pages and
 * a table's columns come with it.
 */
export type LayerPart = { item: number; name: string | null; kind: string; box: Box; own: boolean; pages: LayerPage[]; columns: { field: string; box: Box }[] };

/** A grid of the design: what holds it (the window, a container, a page), its columns and rows, and the containers it lies in. */
export type LayerGrid = { item: number; columns: number; gap: number; box: Box; rows: number[]; inside: number[] };

function boxOf(at: [number, number, number, number]): Box {
  return { x: at[0], y: at[1], width: at[2], height: at[3] };
}

function itemOf(item: string | null): number | null {
  if (item === null || item === "") return null;
  const n = Number(item);
  return Number.isInteger(n) ? n : null;
}

/** The items of every container inside `widget`, its own grid and pages too. */
export function heldBy(widget: Widget): number[] {
  const out = [widget.span.start];
  for (const page of widget.pages) out.push(page.span.start);
  for (const child of [...widget.children, ...widget.pages.flatMap((p) => p.children)]) out.push(...heldBy(child));
  return out;
}

/** Every part of the window, by its item. */
function partsOf(page: Page): Map<number, Widget> {
  const out = new Map<number, Widget>();
  const visit = (widgets: Widget[]) => {
    for (const w of widgets) {
      out.set(w.span.start, w);
      visit(w.children);
      for (const inner of w.pages) visit(inner.children);
    }
  };
  visit(page.widgets);
  for (const sidebar of page.frame.sidebars) visit(sidebar.widgets);
  if (page.frame.footer) visit(page.frame.footer.widgets);
  return out;
}

/** The items of a frame's buttons and menu items, those of submenus too. */
export function commandsOf(frame: Frame | null | undefined): number[] {
  if (!frame) return [];
  const out: number[] = [];
  const visit = (items: Command[]) => {
    for (const c of items) {
      out.push(c.span.start);
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
 * design has (an item it knows), each part once (the first drawn), and each
 * grid with the parts it lies inside, so a part is never dropped into itself.
 * A frame's buttons are known by the window's own frame, or else by its
 * app's (`appFrame`), whose buttons are designed in the app.
 */
export function layerOf(layout: Layout, page: Page, appFrame: Frame | null = null): { parts: LayerPart[]; grids: LayerGrid[] } {
  const known = partsOf(page);
  const own = new Set(commandsOf(page.frame));
  const app = new Set(commandsOf(appFrame));
  const seen = new Set<number>();
  const parts: LayerPart[] = [];
  for (const laid of layout.parts) {
    const item = itemOf(laid.item);
    const command = laid.kind === "command";
    if (item === null || seen.has(item)) continue;
    if (command ? !own.has(item) && !app.has(item) : !known.has(item)) continue;
    seen.add(item);
    const pages = (laid.pages ?? []).flatMap((p) => {
      const at = itemOf(p.item);
      return at === null ? [] : [{ item: at, title: p.title, box: p.at ? boxOf(p.at) : null }];
    });
    const columns = (laid.columns ?? []).map((c) => ({ field: c.field, box: boxOf(c.at) }));
    parts.push({ item, name: laid.name, kind: laid.kind, box: boxOf(laid.at), own: command ? own.has(item) : true, pages, columns });
  }
  const within = new Map<number, number[]>();
  for (const widget of known.values()) {
    for (const held of heldBy(widget)) within.set(held, [...(within.get(held) ?? []), widget.span.start]);
  }
  const grids: LayerGrid[] = [];
  for (const laid of layout.grids) {
    const item = itemOf(laid.item);
    if (item === null) continue;
    grids.push({ item, columns: laid.columns, gap: laid.gap, box: boxOf(laid.at), rows: laid.rows, inside: within.get(item) ?? [] });
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
