// A trend's values and a chart's list of numbers (DESIGN 5.26), alike on
// every renderer: the last `points` numbers, decimals as their text.

/** How many values a trend keeps without `points n`. */
export const TREND_POINTS = 60;

/** A value as a number; decimals and money travel as text. Null when it is none. */
function numberOf(value: unknown): number | null {
  const n = typeof value === "number" ? value : typeof value === "string" && value.trim() !== "" ? Number(value) : NaN;
  return Number.isFinite(n) ? n : null;
}

/** `history` with `value` taken, keeping the last `points`; a value that is no number is not taken. */
export function sampled(history: number[], value: unknown, points: number): number[] {
  const n = numberOf(value);
  if (n === null) return history;
  const next = [...history, n];
  return next.length > points ? next.slice(next.length - points) : next;
}

/** The numbers of a list a chart draws: its last `points`, or all of them. */
export function numbersOf(list: unknown, points: number | null): number[] {
  if (!Array.isArray(list)) return [];
  const numbers = list.map(numberOf).filter((n): n is number => n !== null);
  return points && numbers.length > points ? numbers.slice(numbers.length - points) : numbers;
}
