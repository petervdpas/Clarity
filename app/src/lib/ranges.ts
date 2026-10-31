// The slider and the spin box of U1c apart from the screen
// (conformance/ranges.json): how a key, a drag or a typed number moves a
// value within its range. Decimals and money come as text and stay exact,
// so the sums are done on whole numbers scaled by ten as often as needed.

import type { Literal } from "./generated/Literal";

/** A range as `clarity_spec::widgets::number_range` gives it: null is no end. */
export type Range = { min: string | null; max: string | null; step: string };

/** What the person did to a slider or spin box, which the runtime makes a value of (DESIGN 1.7): a key, a drag to a place, or a typed number. */
export type RangeMove = { key: RangeKey } | { at: string } | { typed: string };

/** The keys a slider and a spin box answer to. */
export type RangeKey = "up" | "down" | "pageUp" | "pageDown" | "home" | "end";

/** Steps a key moves, for the keys that move by steps. */
const KEY_STEPS: Partial<Record<RangeKey, bigint>> = { up: 1n, down: -1n, pageUp: 10n, pageDown: -10n };

type Exact = { n: bigint; scale: number };

function exact(value: string | number): Exact {
  const text = String(value).trim();
  const negative = text.startsWith("-");
  const [whole, part = ""] = text.replace(/^[-+]/, "").split(".");
  const n = BigInt((whole || "0") + part);
  return { n: negative ? -n : n, scale: part.length };
}

/** Whole numbers at a scale of `scale` decimals. */
function at(value: string | number, scale: number): bigint {
  const e = exact(value);
  return e.n * 10n ** BigInt(scale - e.scale);
}

function scaleOf(...values: (string | number | null)[]): number {
  return Math.max(0, ...values.filter((v) => v !== null).map((v) => exact(v as string | number).scale));
}

function text(n: bigint, scale: number): string {
  const negative = n < 0n;
  const digits = (negative ? -n : n).toString().padStart(scale + 1, "0");
  const whole = digits.slice(0, digits.length - scale);
  const part = digits.slice(digits.length - scale).replace(/0+$/, "");
  return (negative ? "-" : "") + whole + (part ? `.${part}` : "");
}

function within(n: bigint, low: bigint | null, high: bigint | null): bigint {
  if (high !== null && n > high) n = high;
  if (low !== null && n < low) n = low;
  return n;
}

/** The value after `key`: a step up or down, ten, or an end; within the range. No value counts as the lowest, or 0. */
export function keyed(value: string | number | null, range: Range, key: RangeKey): string {
  const scale = scaleOf(value, range.min, range.max, range.step);
  const low = range.min === null ? null : at(range.min, scale);
  const high = range.max === null ? null : at(range.max, scale);
  const now = value === null || value === "" ? (low ?? 0n) : at(value, scale);
  const steps = KEY_STEPS[key];
  const next = steps !== undefined ? now + steps * at(range.step, scale) : key === "home" ? (low ?? now) : (high ?? now);
  return text(within(next, low, high), scale);
}

/** The value a slider takes where it is dragged to: the nearest step from the lowest value, at most the highest step in the range. */
export function dragged(where: string | number, range: Range): string {
  const scale = scaleOf(where, range.min, range.max, range.step);
  const low = at(range.min ?? "0", scale);
  const step = at(range.step, scale);
  const from = at(where, scale) - low;
  // Rounded half up, as a whole number of steps.
  let steps = from >= 0n ? (2n * from + step) / (2n * step) : -((-2n * from + step - 1n) / (2n * step));
  const most = range.max === null ? null : (at(range.max, scale) - low) / step;
  steps = within(steps, 0n, most);
  return text(low + steps * step, scale);
}

/** A typed number, kept within the range. */
export function typedIn(value: string | number, range: Range): string {
  const scale = scaleOf(value, range.min, range.max);
  const low = range.min === null ? null : at(range.min, scale);
  const high = range.max === null ? null : at(range.max, scale);
  return text(within(at(value, scale), low, high), scale);
}

/** A value as the window holds it: a whole number as a number, a decimal as text. */
export function rangeValue(value: string, whole: boolean): string | number {
  return whole ? Number(value) : value;
}

/** The range of a slider or a spin box, as `clarity_spec::widgets::number_range` gives it: as written, else the bound field's own ends; a slider without either runs from 0 to 100. */
export function numberRange(
  p: { kind: string; min: string | null; max: string | null; step: string | null },
  modifiers?: { min: Literal | null; max: Literal | null },
): Range {
  const own = (l: Literal | null | undefined) => (l?.kind === "number" ? l.value : null);
  let min = p.min ?? own(modifiers?.min);
  let max = p.max ?? own(modifiers?.max);
  if (p.kind === "slider") {
    min ??= "0";
    max ??= "100";
  }
  return { min, max, step: p.step ?? "1" };
}

/** The range key a key press is, when it is one. */
export function rangeKey(key: string): RangeKey | null {
  return RANGE_KEYS[key] ?? null;
}

const RANGE_KEYS: Record<string, RangeKey> = {
  ArrowUp: "up",
  ArrowRight: "up",
  ArrowDown: "down",
  ArrowLeft: "down",
  PageUp: "pageUp",
  PageDown: "pageDown",
  Home: "home",
  End: "end",
};
