// Columns of a table the person resizes: drag the right edge of a header
// cell, double-click it to give the table its automatic widths again.
// A table stays automatic until a column is first dragged; from then on
// its widths are fixed and remembered in this browser, by column name, so
// a column that comes and goes keeps the others as they were.
//
//   <table use:resizableColumns={"clarity.columns.entityFields"}>
//
// A header cell's name is its data-column attribute, or else its text.

/** The narrowest a column is dragged to. */
export const MIN_WIDTH = 40;

/** The width of a column `width` wide after a drag of `dx`. */
export function resized(width: number, dx: number): number {
  return Math.max(MIN_WIDTH, Math.round(width + dx));
}

/** Saved widths by column name; anything that is not such a map reads as none. */
export function readWidths(text: string | null): Record<string, number> {
  if (!text) return {};
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return {};
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) return {};
  const widths: Record<string, number> = {};
  for (const [name, width] of Object.entries(value)) {
    if (typeof width === "number" && Number.isFinite(width) && width > 0) widths[name] = Math.round(width);
  }
  return widths;
}

/** Each column's width: saved under its name, or as it was shown. */
export function widthsFor(names: string[], shown: number[], saved: Record<string, number>): number[] {
  return names.map((name, i) => (name && saved[name] !== undefined ? saved[name] : shown[i]));
}

function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function store(key: string, widths: Record<string, number> | null) {
  try {
    if (widths) localStorage.setItem(key, JSON.stringify(widths));
    else localStorage.removeItem(key);
  } catch {
    // Without storage the widths last until the table is shown again.
  }
}

/** Makes the columns of `table` resizable, remembered under `key`. */
export function resizableColumns(table: HTMLTableElement, key: string) {
  let storageKey = key;
  let observer: MutationObserver | null = null;

  const headers = () => [...(table.tHead?.rows[0]?.cells ?? [])] as HTMLTableCellElement[];
  const nameOf = (cell: HTMLTableCellElement) => cell.dataset.column ?? cell.textContent?.trim() ?? "";

  function automatic() {
    table.classList.remove("sized");
    table.style.tableLayout = "";
    table.style.width = "";
    for (const cell of headers()) cell.style.width = "";
  }

  function fix(widths: number[]) {
    const cells = headers();
    cells.forEach((cell, i) => (cell.style.width = `${widths[i]}px`));
    table.style.width = `${widths.reduce((a, b) => a + b, 0)}px`;
    table.style.tableLayout = "fixed";
    table.classList.add("sized");
  }

  function save() {
    const widths: Record<string, number> = { ...readWidths(load(storageKey)) };
    for (const cell of headers()) {
      const name = nameOf(cell);
      if (name) widths[name] = Math.round(cell.offsetWidth);
    }
    store(storageKey, widths);
  }

  function grip(cell: HTMLTableCellElement, index: number) {
    const handle = document.createElement("span");
    handle.className = "column-grip";
    handle.title = "Drag to resize; double-click for automatic widths";
    handle.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      const cells = headers();
      const widths = cells.map((c) => c.offsetWidth);
      fix(widths);
      const start = { x: e.clientX, width: widths[index] };
      handle.classList.add("dragging");
      const move = (ev: PointerEvent) => {
        widths[index] = resized(start.width, ev.clientX - start.x);
        fix(widths);
      };
      const up = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", up);
        handle.classList.remove("dragging");
        save();
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", up);
    });
    handle.addEventListener("dblclick", (e) => {
      e.stopPropagation();
      store(storageKey, null);
      automatic();
    });
    cell.style.position = "relative";
    cell.append(handle);
  }

  /** Lays the table out again: automatic, then the saved widths if any; a grip on every column but the last. */
  function setup() {
    observer?.disconnect();
    for (const old of table.querySelectorAll(":scope > thead .column-grip")) old.remove();
    automatic();
    const cells = headers();
    const saved = readWidths(load(storageKey));
    if (Object.keys(saved).length > 0 && cells.length > 0) {
      fix(widthsFor(cells.map(nameOf), cells.map((c) => c.offsetWidth), saved));
    }
    cells.slice(0, -1).forEach(grip);
    if (table.tHead) observer?.observe(table.tHead, { childList: true, subtree: true, characterData: true });
  }

  // Columns that come and go (a header cell added or taken away) lay the table out again.
  observer = new MutationObserver(setup);
  setup();

  return {
    update(next: string) {
      if (next === storageKey) return;
      storageKey = next;
      setup();
    },
    destroy() {
      observer?.disconnect();
    },
  };
}
