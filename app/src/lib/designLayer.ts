// The designer's layer over the window Clarity.UI draws (DESIGN 8.1): a
// box for each part and each grid, placed where the library reports them
// (`Layout`, its `layout` step), each known by where it is written in the
// project (its item, the offset the designer's edits name). Pure, so the
// placing is tested without a browser.

import type { Page } from "@clarity/ui/generated/Page.ts";
import type { Widget } from "@clarity/ui/generated/Widget.ts";
import type { Layout } from "./clarityWindow";

export type Box = { x: number; y: number; width: number; height: number };

/** A part of the design as the layer shows it. */
export type LayerPart = { item: number; name: string | null; kind: string; box: Box };

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

/**
 * The layer's parts and grids from what Clarity.UI reports: only what the
 * design has (an item it knows), each part once (the first drawn), and each
 * grid with the parts it lies inside, so a part is never dropped into itself.
 */
export function layerOf(layout: Layout, page: Page): { parts: LayerPart[]; grids: LayerGrid[] } {
  const known = partsOf(page);
  const seen = new Set<number>();
  const parts: LayerPart[] = [];
  for (const laid of layout.parts) {
    const item = itemOf(laid.item);
    if (item === null || !known.has(item) || seen.has(item)) continue;
    seen.add(item);
    parts.push({ item, name: laid.name, kind: laid.kind, box: boxOf(laid.at) });
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

/** The cells of a grid, each column by each row, in its own pixels from its top left. */
export function cellsOf(grid: LayerGrid): Box[] {
  const column = (grid.box.width - grid.gap * (grid.columns - 1)) / grid.columns;
  const out: Box[] = [];
  let y = 0;
  for (const height of grid.rows) {
    for (let c = 0; c < grid.columns; c++) out.push({ x: c * (column + grid.gap), y, width: column, height });
    y += height + grid.gap;
  }
  return out;
}

/** Where a part dropped at `col`, `row`, `width` columns by `height` rows would be, in the grid's own pixels; rows past the last are 36 high. */
export function placeIn(grid: LayerGrid, at: { col: number; row: number; width: number; height: number }): Box {
  const ROW = 36;
  const column = (grid.box.width - grid.gap * (grid.columns - 1)) / grid.columns;
  const rowHeight = (r: number) => grid.rows[r - 1] ?? ROW;
  let top = 0;
  for (let r = 1; r < at.row; r++) top += rowHeight(r) + grid.gap;
  let height = 0;
  for (let r = at.row; r < at.row + at.height; r++) height += rowHeight(r) + (r > at.row ? grid.gap : 0);
  return { x: (at.col - 1) * (column + grid.gap), y: top, width: at.width * column + (at.width - 1) * grid.gap, height };
}
