// How a table's rows are shown in order, apart from the screen
// (conformance/table-order.json): text ignoring case, numbers as numbers, a
// choice by the label it shows, an empty value last either way, and equal
// rows in the order of the data. Rows keep their place in the data; only
// the order they are shown in changes.

import type { JsonValue } from "../generated/serde_json/JsonValue";
import type { CellKind } from "./Table.svelte";

export type Order = { field: string; desc: boolean } | null;

/** What a click on the header of `field` makes of the order: by it, then the other way. */
export function nextOrder(order: Order, field: string): Order {
  return order?.field === field ? { field, desc: !order.desc } : { field, desc: false };
}

type Sorted = { kind: CellKind; shown?: (value: JsonValue | undefined) => string };

/** What a value sorts by: a number, or text; null when it is empty. */
function key(column: Sorted, value: JsonValue | undefined): number | string | null {
  if (value === null || value === undefined || value === "") return null;
  if (column.kind === "choice") {
    const label = column.shown ? column.shown(value) : String(value);
    return label === "" ? null : label;
  }
  if (column.kind === "number") {
    const n = Number(value);
    return Number.isNaN(n) ? null : n;
  }
  if (column.kind === "bool") return value === true ? 1 : 0;
  return String(value);
}

const collator = new Intl.Collator(undefined, { sensitivity: "base", numeric: false });

/** The places in the data of `rows`, in the order they are shown. */
export function shownOrder(rows: Record<string, JsonValue>[], columns: ({ field: string } & Sorted)[], order: Order): number[] {
  const places = rows.map((_, i) => i);
  const column = order ? columns.find((c) => c.field === order.field) : undefined;
  if (!order || !column) return places;
  const keys = rows.map((row) => key(column, row[column.field]));
  return places.sort((a, b) => {
    const ka = keys[a];
    const kb = keys[b];
    if (ka === null || kb === null) return ka === kb ? a - b : ka === null ? 1 : -1;
    const by = typeof ka === "number" && typeof kb === "number" ? ka - kb : collator.compare(String(ka), String(kb));
    return (order.desc ? -by : by) || a - b;
  });
}
