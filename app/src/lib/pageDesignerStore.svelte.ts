// State of the page designer: selection and pointer-driven dragging on the
// snap grid. A drag is a new widget from the palette, a field from the data
// list, a widget being moved, or a widget being resized. While dragging, the
// pointer is translated into a cell of the innermost grid under it.

import type { Dock } from "./generated/Dock";
import type { Preview } from "./generated/Preview";
import type { RegionKind } from "./generated/RegionKind";
import type { WidgetKind } from "./generated/WidgetKind";
import type { WidgetProps } from "./generated/WidgetProps";
import { GAP, rowAt } from "./pages";


export type PageDragSource =
  | { kind: "new"; widget: WidgetKind; width: number; height: number; label: string }
  | { kind: "field"; widget: WidgetKind; bind: string; caption: string; width: number; height: number; label: string }
  | { kind: "move"; item: number; from: number; props: WidgetProps; grab: { dc: number; dr: number }; label: string }
  | { kind: "resize"; item: number; props: WidgetProps; label: string }
  | { kind: "region"; region: RegionKind; docks: Dock[]; label: string };

/** Where a drop would land: a container and the rectangle in its grid. */
export type Placement = { container: number; columns: number; col: number; row: number; width: number; height: number };

/** What the text panel shows of a window: its design or its code behind. */
export type PanelView = "design" | "code";

const DRAG_THRESHOLD = 4;

type Pending = { source: PageDragSource; x: number; y: number; onclick: () => void };

class PageDesigner {
  selected = $state<number | null>(null);
  activeTabs = $state<Record<number, number>>({});
  error = $state<string | null>(null);
  /** An edit is being written. */
  saving = $state(false);
  /** Show the snap grid cells while designing. */
  showGrid = $state(true);
  /** The text panel: open or not, which view, and the handler to show in the code. */
  code = $state<{ open: boolean; view: PanelView; focus: string | null }>({ open: false, view: "code", focus: null });
  /** The project as the Design text being typed would make it; the canvas shows it until it is saved or reverted. */
  draft = $state<Preview | null>(null);

  drag = $state<PageDragSource | null>(null);
  hover = $state<Placement | null>(null);
  /** The window edge a dragged region would dock to. */
  dock = $state<Dock | null>(null);
  pointer = $state({ x: 0, y: 0 });

  #pending: Pending | null = null;
  #canvas: HTMLElement | null = null;
  #ondrop: ((source: PageDragSource, placement: Placement) => void) | null = null;
  #ondock: ((region: RegionKind, dock: Dock) => void) | null = null;

  attach(
    canvas: HTMLElement,
    ondrop: (source: PageDragSource, placement: Placement) => void,
    ondock: (region: RegionKind, dock: Dock) => void,
  ): () => void {
    this.#canvas = canvas;
    this.#ondrop = ondrop;
    this.#ondock = ondock;
    const move = (e: PointerEvent) => this.#move(e);
    const up = (e: PointerEvent) => this.#up(e);
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") this.cancel();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("keydown", key);
      this.#canvas = null;
      this.#ondrop = null;
      this.#ondock = null;
    };
  }

  press(e: PointerEvent, source: PageDragSource, onclick: () => void = () => {}) {
    if (e.button !== 0) return;
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    e.preventDefault();
    e.stopPropagation();
    this.#pending = { source, x: e.clientX, y: e.clientY, onclick };
  }

  /** A part double-clicked on the window, whose main event's code is to open; `n` changes for each. */
  partCode = $state<{ item: number; n: number } | null>(null);

  /** Asks for the code of the part at `item`, as a double click on it does. */
  openPartCode(item: number) {
    this.selected = item;
    this.partCode = { item, n: (this.partCode?.n ?? 0) + 1 };
  }

  /** An app's menu item or button double-clicked in a window, which is edited in the app; `n` changes for each. */
  appCode = $state<{ item: number; n: number } | null>(null);

  /** Asks to open the app's designer at its command `item`, with that command's code. */
  openInApp(item: number) {
    this.appCode = { item, n: (this.appCode?.n ?? 0) + 1 };
  }

  /** Opens the code behind panel at a handler, or at the top. */
  showCode(handler: string | null) {
    this.code = { open: true, view: "code", focus: handler };
  }

  /** Opens the text panel at `view`, or closes it when it already shows that view. */
  togglePanel(view: PanelView) {
    const showing = this.code.open && this.code.view === view;
    this.code = { open: !showing, view, focus: null };
  }

  cancel() {
    this.#pending = null;
    this.drag = null;
    this.hover = null;
    this.dock = null;
  }

  #move(e: PointerEvent) {
    this.pointer = { x: e.clientX, y: e.clientY };
    const pending = this.#pending;
    if (pending && !this.drag) {
      if (Math.hypot(e.clientX - pending.x, e.clientY - pending.y) < DRAG_THRESHOLD) return;
      this.drag = pending.source;
      this.error = null;
    }
    if (!this.drag) return;
    if (this.drag.kind === "region") {
      this.dock = this.#dockAt(e.clientX, e.clientY, this.drag.docks);
      return;
    }
    this.#switchTabUnderPointer(e);
    this.hover = this.#placement(e.clientX, e.clientY, this.drag);
  }

  #up(e: PointerEvent) {
    const pending = this.#pending;
    const drag = this.drag;
    const hover = this.hover;
    const dock = this.dock;
    this.cancel();
    if (drag?.kind === "region") {
      if (dock) this.#ondock?.(drag.region, dock);
    } else if (drag && hover) this.#ondrop?.(drag, hover);
    else if (pending && !drag && e.button === 0) pending.onclick();
  }

  #switchTabUnderPointer(e: PointerEvent) {
    const target = document.elementFromPoint(e.clientX, e.clientY);
    const tab = target instanceof Element ? target.closest<HTMLElement>("[data-tab-switch]") : null;
    if (!tab?.dataset.tabSwitch) return;
    const [tabs, index] = tab.dataset.tabSwitch.split(":").map(Number);
    if (this.activeTabs[tabs] !== index) this.activeTabs[tabs] = index;
  }

  /** The dock zone under the pointer, among the edges the region accepts. */
  #dockAt(x: number, y: number, docks: Dock[]): Dock | null {
    const hit = document.elementFromPoint(x, y);
    const zone = hit instanceof Element ? hit.closest<HTMLElement>("[data-dock]") : null;
    const dock = zone?.dataset.dock as Dock | undefined;
    return dock && docks.includes(dock) ? dock : null;
  }

  /** The innermost grid under the pointer that the dragged widget may enter. */
  #gridAt(x: number, y: number, source: Exclude<PageDragSource, { kind: "region" }>): HTMLElement | null {
    if (!this.#canvas) return null;
    if (source.kind === "resize") {
      // A resize stays in the widget's own container.
      const widget = this.#canvas.querySelector(`[data-widget="${source.item}"]`);
      return widget?.parentElement?.closest<HTMLElement>("[data-grid]") ?? null;
    }
    const moving = source.kind === "move" ? source.item : null;
    for (const el of document.elementsFromPoint(x, y)) {
      const grid = el.closest<HTMLElement>("[data-grid]");
      if (!grid || !this.#canvas.contains(grid)) continue;
      if (moving !== null && grid.closest(`[data-widget="${moving}"]`)) continue;
      return grid;
    }
    return null;
  }

  #placement(x: number, y: number, source: PageDragSource): Placement | null {
    if (source.kind === "region") return null;
    const grid = this.#gridAt(x, y, source);
    if (!grid) return null;
    const columns = Number(grid.dataset.columns);
    const container = Number(grid.dataset.grid);
    const rect = grid.getBoundingClientRect();
    const colWidth = (rect.width - GAP * (columns - 1)) / columns;
    const cellCol = Math.min(columns, Math.max(1, Math.floor((x - rect.left) / (colWidth + GAP)) + 1));
    const cellRow = rowAt(grid, y - rect.top, rect.height);

    if (source.kind === "resize") {
      const p = source.props;
      return {
        container,
        columns,
        col: p.col,
        row: p.row,
        width: Math.min(columns - p.col + 1, Math.max(1, cellCol - p.col + 1)),
        height: Math.max(1, cellRow - p.row + 1),
      };
    }

    const size =
      source.kind === "move"
        ? { width: source.props.width, height: source.props.height }
        : { width: source.width, height: source.height };
    const width = Math.min(size.width, columns);
    const grab = source.kind === "move" ? source.grab : { dc: 0, dr: 0 };
    const col = Math.min(columns - width + 1, Math.max(1, cellCol - grab.dc));
    const row = Math.max(1, cellRow - grab.dr);
    return { container, columns, col, row, width, height: size.height };
  }
}

export const pageDesigner = new PageDesigner();
