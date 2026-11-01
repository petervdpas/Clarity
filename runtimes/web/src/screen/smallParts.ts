// The small parts of U1b apart from the screen (conformance/small-parts.json):
// what a link bound to a field opens, and how full a progress bar is.

/** What a link opens for a value it binds: an email address to write to, an address with a scheme as it is, else nothing. */
export function linkAddress(value: unknown): string | null {
  const text = typeof value === "string" ? value.trim() : "";
  if (text === "") return null;
  if (text.includes("@") && !text.includes(":")) return `mailto:${text}`;
  return /^[a-z][a-z0-9+.-]*:/i.test(text) ? text : null;
}

/** How full a progress bar is, from 0 to 1, for `value` of `max` (100 without one). */
export function progressFull(value: unknown, max: string | null): number {
  const n = typeof value === "number" ? value : typeof value === "string" && value.trim() !== "" ? Number(value) : NaN;
  const top = max === null ? 100 : Number(max);
  if (!Number.isFinite(n) || !(top > 0)) return 0;
  return Math.min(1, Math.max(0, n / top));
}
