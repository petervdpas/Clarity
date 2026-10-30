// Moving rows by hand (DESIGN 5.30): the rows of a list, table, tree or tree
// table that reorders are dragged with the pointer, and the chosen row moves
// with Alt and an arrow key. Rows carry `data-row`, their place in the list.
// Pointer events, not the browser's drag and drop, so it works alike in every
// desktop window and browser.

import type { Move } from "./reorder.ts";

/** What a container of rows needs to move them. */
export type RowDrag = {
  /** Whether rows can be moved now (not in the designer, not disabled). */
  enabled: boolean;
  /** A tree: a drop on the middle of a row puts the dragged row under it. */
  tree: boolean;
  /** A row was dropped somewhere else. */
  onmove: (move: Move) => void;
};

/** How far the pointer goes before a press is a drag, in pixels. */
const START = 4;

/** Where a drop at `y` on a row lands: before or after it, or (in a tree) into it. */
export function dropWhere(y: number, top: number, height: number, tree: boolean): "before" | "after" | "into" {
  const part = (y - top) / Math.max(height, 1);
  if (!tree) return part < 0.5 ? "before" : "after";
  return part < 0.25 ? "before" : part > 0.75 ? "after" : "into";
}

/** The move Alt and an arrow key ask for on the chosen row, or none. */
export function keyMove(e: KeyboardEvent, row: number | null): Move | null {
  if (!e.altKey || e.ctrlKey || e.metaKey || row === null) return null;
  const keys = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" } as const;
  const key = keys[e.key as keyof typeof keys];
  return key ? { row, key } : null;
}

const rowAt = (target: EventTarget | null) => (target instanceof Element ? target.closest<HTMLElement>("[data-row]") : null);

/** A Svelte action on the element that holds the rows. */
export function rowDrag(node: HTMLElement, given: RowDrag) {
  let options = given;
  let pressed: { row: number; x: number; y: number; id: number } | null = null;
  let dragging = false;
  let over: { el: HTMLElement; target: number; where: "before" | "after" | "into" } | null = null;

  const mark = (next: typeof over) => {
    if (over && over.el !== next?.el) delete over.el.dataset.drop;
    over = next;
    if (over) over.el.dataset.drop = over.where;
  };

  const down = (e: PointerEvent) => {
    if (!options.enabled || e.button !== 0) return;
    // A toggle, a cell's input or a button keeps its own press.
    if (e.target instanceof Element && e.target.closest("button, input, select, textarea, [contenteditable]")) return;
    const row = rowAt(e.target);
    if (!row) return;
    pressed = { row: Number(row.dataset.row), x: e.clientX, y: e.clientY, id: e.pointerId };
  };

  const move = (e: PointerEvent) => {
    if (!pressed || e.pointerId !== pressed.id) return;
    if (!dragging && Math.hypot(e.clientX - pressed.x, e.clientY - pressed.y) < START) return;
    if (!dragging) {
      dragging = true;
      node.setPointerCapture?.(e.pointerId);
      node.classList.add("dragging-row");
    }
    const el = rowAt(document.elementFromPoint?.(e.clientX, e.clientY) ?? null);
    if (!el || !node.contains(el)) return mark(null);
    const box = el.getBoundingClientRect();
    mark({ el, target: Number(el.dataset.row), where: dropWhere(e.clientY, box.top, box.height, options.tree) });
  };

  const up = (e: PointerEvent) => {
    if (!pressed || e.pointerId !== pressed.id) return;
    const row = pressed.row;
    const drop = dragging ? over : null;
    pressed = null;
    dragging = false;
    node.classList.remove("dragging-row");
    node.releasePointerCapture?.(e.pointerId);
    mark(null);
    if (drop) options.onmove({ row, where: drop.where, target: drop.target });
  };

  const cancel = () => {
    pressed = null;
    dragging = false;
    node.classList.remove("dragging-row");
    mark(null);
  };

  node.addEventListener("pointerdown", down);
  node.addEventListener("pointermove", move);
  node.addEventListener("pointerup", up);
  node.addEventListener("pointercancel", cancel);
  return {
    update(next: RowDrag) {
      options = next;
      if (!next.enabled) cancel();
    },
    destroy() {
      node.removeEventListener("pointerdown", down);
      node.removeEventListener("pointermove", move);
      node.removeEventListener("pointerup", up);
      node.removeEventListener("pointercancel", cancel);
    },
  };
}
