// Charts of U1c, as every renderer draws them (conformance/charts.json):
// a list's rows grouped by a field, each group counted, summed or averaged,
// the groups in the order of their values; the axis of a bar or line chart
// in steps of 1, 2, 2.5 or 5 times a power of ten; a pie's slices in the
// palette's fourth row of colors.

import { PALETTE } from "../colors.ts";

/** What a chart groups its rows by, for the order of the groups. */
export type ByKind = "text" | "number" | "date" | "enum" | "bool";
export type Measure = "count" | "sum" | "average";
export type ChartRow = { by: unknown; of: unknown };
export type Group = { key: unknown; value: number };

const round12 = (n: number) => Math.round(n * 1e12) / 1e12;

function numberOf(value: unknown): number | null {
  if (value === null || value === undefined || value === "") return null;
  const n = Number(value);
  return Number.isNaN(n) ? null : n;
}

/** The groups of a chart's rows, in the order of their values. */
export function groupsOf(rows: ChartRow[], by: ByKind, measure: Measure, order: unknown[] = []): Group[] {
  const groups = new Map<unknown, { count: number; sum: number; numbers: number }>();
  for (const row of rows) {
    if (row.by === null || row.by === undefined || row.by === "") continue;
    const g = groups.get(row.by) ?? { count: 0, sum: 0, numbers: 0 };
    g.count += 1;
    const n = numberOf(row.of);
    if (n !== null) {
      g.sum += n;
      g.numbers += 1;
    }
    groups.set(row.by, g);
  }
  const keys = [...groups.keys()];
  const before = (a: unknown, b: unknown): number => {
    if (by === "number") return Number(a) - Number(b);
    if (by === "enum") return order.indexOf(a) - order.indexOf(b);
    if (by === "bool") return Number(a === true) - Number(b === true);
    const [sa, sb] = [String(a), String(b)];
    if (by === "date") return sa < sb ? -1 : sa > sb ? 1 : 0;
    const [la, lb] = [sa.toLowerCase(), sb.toLowerCase()];
    if (la !== lb) return la < lb ? -1 : 1;
    return sa < sb ? -1 : sa > sb ? 1 : 0;
  };
  return keys.sort(before).map((key) => {
    const g = groups.get(key)!;
    const value = measure === "count" ? g.count : measure === "sum" ? g.sum : g.numbers > 0 ? g.sum / g.numbers : 0;
    return { key, value: round12(value) };
  });
}

/** The axis of a bar or line chart for its lowest and highest number. */
export function scaleOf(low: number, high: number): { bottom: number; top: number; step: number } {
  const lo = Math.min(low, 0);
  let hi = Math.max(high, 0);
  if (lo === 0 && hi === 0) hi = 1;
  const raw = (hi - lo) / 5;
  const power = 10 ** Math.floor(Math.log10(raw));
  const rest = round12(raw / power);
  const factor = rest <= 1 ? 1 : rest <= 2 ? 2 : rest <= 2.5 ? 2.5 : rest <= 5 ? 5 : 10;
  const step = round12(factor * power);
  const bottom = round12(Math.floor(round12(lo / step)) * step);
  const top = round12(Math.ceil(round12(hi / step)) * step);
  return { bottom: bottom === 0 ? 0 : bottom, top, step };
}

/** The colors of a pie's slices: none for a slice of 0 or less. */
export function sliceColors(values: number[]): (string | null)[] {
  let next = 0;
  return values.map((v) => (v > 0 ? PALETTE[3][next++ % PALETTE[3].length] : null));
}
