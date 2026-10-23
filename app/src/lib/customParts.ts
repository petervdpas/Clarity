// Custom parts in a running application (DESIGN 5.19 and 6.5): an outside
// library draws the part in a native window the shell keeps over the place
// the renderer gives it. The renderer works out where the part is, whether
// it shows, and which of its values to give it.

import type { Literal } from "./generated/Literal";
import type { Param } from "./generated/Param";
import type { TypeRefKind } from "./generated/TypeRefKind";
import type { JsonValue } from "./generated/serde_json/JsonValue";

/** A rectangle as the page measures it. */
export type Box = { left: number; top: number; width: number; height: number };

/** Where a part is in the webview, in its pixels. */
export type Place = { x: number; y: number; width: number; height: number };

/** A value written in the window as the part takes it: a whole number as a number, other numbers as text, a text in the person's language. */
function written(value: Literal, words: (text: string) => string): JsonValue {
  switch (value.kind) {
    case "number":
      return /^-?\d+$/.test(value.value) ? Number(value.value) : value.value;
    case "str":
      return words(value.value);
    case "bool":
      return value.value;
    case "ident":
      return value.value;
  }
}

/** A part's values: those written on its line, under those code set. */
export function shownValues(
  writtenValues: Record<string, Literal | undefined>,
  set: Record<string, JsonValue> | null | undefined,
  words: (text: string) => string,
): Record<string, JsonValue> {
  const out: Record<string, JsonValue> = {};
  for (const [name, value] of Object.entries(writtenValues)) {
    if (value) out[name] = written(value, words);
  }
  return { ...out, ...(set ?? {}) };
}

/** The values that differ from those given before, to give again. */
export function changedValues(before: Record<string, JsonValue> | null, now: Record<string, JsonValue>): [string, JsonValue][] {
  return Object.entries(now).filter(([name, value]) => !before || JSON.stringify(before[name]) !== JSON.stringify(value));
}

/** A rectangle of the page in the webview's pixels. */
export function placeOf(box: Box, ratio: number): Place {
  return { x: Math.round(box.left * ratio), y: Math.round(box.top * ratio), width: Math.round(box.width * ratio), height: Math.round(box.height * ratio) };
}

/** Whether an overlay of Clarity's (a dialog, a popover, a menu, a list that opened) lies over the part. */
export function covered(part: Box, overlays: Box[]): boolean {
  return overlays.some(
    (o) => o.width > 0 && o.height > 0 && o.left < part.left + part.width && part.left < o.left + o.width && o.top < part.top + part.height && part.top < o.top + o.height,
  );
}

/** The class an element of Clarity's overlays carries, which a custom part hides under. */
export const OVERLAY_CLASS = "clarity-overlay";

/** What is typed for a property of `type` as the value written on the part's line; null for none or one that does not fit. */
export function writtenValue(type: TypeRefKind, typed: string): Literal | null {
  const text = typed.trim();
  if (text === "") return null;
  switch (type.kind) {
    case "integer":
    case "duration":
      return /^-?\d+$/.test(text) ? { kind: "number", value: text } : null;
    case "decimal":
    case "money":
      return /^-?\d+(\.\d+)?$/.test(text) ? { kind: "number", value: text } : null;
    case "bool":
      return text === "true" || text === "false" ? { kind: "bool", value: text === "true" } : null;
    case "named":
      return /^[A-Za-z_]\w*$/.test(text) ? { kind: "ident", value: text } : null;
    default:
      return { kind: "str", value: text };
  }
}

/** A value written on the part's line, as the properties panel shows it. */
export function literalShown(value: Literal | undefined): string {
  if (!value) return "";
  return String(value.value);
}

/** The values a tell gives, as its handler takes them: `points: integer, by: text`. */
export function paramsText(params: Param[]): string {
  return params.map((p) => `${p.name.name}: ${typeText(p.ty.kind)}`).join(", ");
}

function typeText(type: TypeRefKind): string {
  switch (type.kind) {
    case "named":
      return type.name;
    case "list":
      return `list<${type.item}>`;
    case "dateTime":
      return "datetime";
    default:
      return type.kind;
  }
}
