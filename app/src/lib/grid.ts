// The layout of a window's grid, the same on every stack (design section
// 2): its sizes, kept equal to clarity_spec::layout by a test, and which
// row a point on the screen is in.

import type { Edit } from "./generated/Edit";
import type { Widget } from "./generated/Widget";

/** Height of one grid row and the gap between cells, in pixels. */
export const ROW_HEIGHT = 36;
export const GAP = 8;
/** The size a window opens at on every stack, which the designer shows, and the space around its grid. */
export const WINDOW_WIDTH = 1100;
export const WINDOW_HEIGHT = 760;

/** The smallest and largest size a window may say, in pixels. */
export const WINDOW_MIN_WIDTH = 200;
export const WINDOW_MIN_HEIGHT = 150;
export const WINDOW_MAX_WIDTH = 3840;
export const WINDOW_MAX_HEIGHT = 2160;

/** An edge of a window the designer drags to size it. */
export type WindowEdge = "right" | "bottom" | "corner";

/**
 * The size of a window whose `edge` was dragged `dx` by `dy` pixels on the
 * screen, from `start`, while the window is shown at `scale`: whole pixels,
 * within the sizes a window may have.
 */
export function resized(start: { width: number; height: number }, edge: WindowEdge, dx: number, dy: number, scale: number): { width: number; height: number } {
  const within = (value: number, low: number, high: number) => Math.min(high, Math.max(low, Math.round(value)));
  const width = edge === "bottom" ? start.width : start.width + dx / scale;
  const height = edge === "right" ? start.height : start.height + dy / scale;
  return { width: within(width, WINDOW_MIN_WIDTH, WINDOW_MAX_WIDTH), height: within(height, WINDOW_MIN_HEIGHT, WINDOW_MAX_HEIGHT) };
}

/** The size a window opens at: its own (`size 320 x 460`), or the standard one. */
export function windowSize(page: { size: { width: number; height: number } | null }): { width: number; height: number } {
  return page.size ?? { width: WINDOW_WIDTH, height: WINDOW_HEIGHT };
}
export const CONTENT_PADDING = 12;

/**
 * The row of `grid` at `offset` pixels from its top, as the screen shows
 * it: rows that grow are taller than the others, and the window may be
 * scaled to fit. Below the last row, rows of the usual height follow.
 */
export function rowAt(grid: HTMLElement, offset: number, shownHeight: number): number {
  const scale = grid.offsetHeight > 0 ? shownHeight / grid.offsetHeight : 1;
  const tracks = getComputedStyle(grid)
    .gridTemplateRows.split(" ")
    .map((t) => parseFloat(t))
    .filter((t) => !Number.isNaN(t));
  return rowIn(tracks, offset / scale);
}

/** The row at `offset` among rows of the heights `tracks`, with GAP between them. */
export function rowIn(tracks: number[], offset: number): number {
  let top = 0;
  for (let i = 0; i < tracks.length; i++) {
    const bottom = top + tracks[i] + GAP;
    if (offset < bottom) return i + 1;
    top = bottom;
  }
  return tracks.length + Math.max(0, Math.floor((offset - top) / (ROW_HEIGHT + GAP))) + 1;
}

/**
 * A part's columns on a grid of `from` columns, as they fall on one of
 * `to`: both edges scale, so parts keep their order and stay apart. A part
 * is never narrower than one column.
 */
export function scaledColumns(col: number, width: number, from: number, to: number): { col: number; width: number } {
  const start = Math.round(((col - 1) * to) / from) + 1;
  const end = Math.round(((col - 1 + width) * to) / from);
  return { col: start, width: Math.max(1, end - start + 1) };
}

/**
 * The edits that resize `widget` to `width` by `height`. A container's
 * parts sit on its own grid, as many columns as it is wide, so they scale
 * across with it (and theirs with them): the content keeps its shape.
 */
export function resizeEdits(widget: Widget, width: number, height: number): Edit[] {
  const edits: Edit[] = [{ op: "updateWidget", item: widget.span.start, container: null, props: { ...widget.props, width, height } }];
  scaleInside(widget, widget.props.width, width, edits);
  return edits;
}

function scaleInside(widget: Widget, from: number, to: number, edits: Edit[]) {
  if (from === to) return;
  for (const child of [...widget.children, ...widget.pages.flatMap((p) => p.children)]) {
    const { col, width } = scaledColumns(child.props.col, child.props.width, from, to);
    if (col === child.props.col && width === child.props.width) continue;
    edits.push({ op: "updateWidget", item: child.span.start, container: null, props: { ...child.props, col, width } });
    scaleInside(child, child.props.width, width, edits);
  }
}
