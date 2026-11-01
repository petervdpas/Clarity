// A part from its element's attributes (DESIGN 7.8): its kind's defaults
// (Rust's catalog, generated/part-defaults.json), and what the attributes
// change, in Clarity's own words, as crates/clarity-gen/src/web_windows.rs
// writes them.

import defaults from "../generated/part-defaults.json";
import type { Event } from "../generated/Event.ts";
import type { Handler } from "../generated/Handler.ts";
import type { Literal } from "../generated/Literal.ts";
import type { Widget } from "../generated/Widget.ts";
import type { WidgetKind } from "../generated/WidgetKind.ts";
import type { WidgetProps } from "../generated/WidgetProps.ts";

/** The words Clarity writes for a field of a part's props, where they differ from its name. */
const FIELDS: Record<string, keyof WidgetProps> = {
  key: "shortcut",
  taborder: "tabOrder",
  childList: "children",
  opens: "onClick",
};

const EVENTS: readonly Event[] = ["click", "change", "enter", "leave", "select", "open", "pointer", "key", "close", "return", "start", "submit", "draw", "drag", "drop", "bend", "next", "finish", "reorder", "tick", "unconnected"];

/** A part of `kind` as its element's attributes write it, at `offset`. */
export function partOf(kind: WidgetKind, given: Record<string, unknown>, offset: number): Widget {
  const props = structuredClone((defaults as Record<string, WidgetProps>)[kind]) as WidgetProps;
  const handlers: Handler[] = [];
  // A part one row high writes no `rows`.
  props.height = 1;
  for (const [word, value] of Object.entries(given)) {
    if (word === "children") continue;
    if (word === "at") {
      const [col, row] = String(value).split(",");
      props.col = Number(col);
      props.row = Number(row);
    } else if (word === "span") props.width = Number(value);
    else if (word === "rows") props.height = Number(value);
    else if (word.startsWith("on") && (EVENTS as readonly string[]).includes(word.slice(2)) && typeof value === "string") handlers.push(handlerOf(word.slice(2) as Event, value));
    else (props as Record<string, unknown>)[FIELDS[word] ?? word] = value;
  }
  props.handlers = handlers;
  return { props, children: [], pages: [], menu: null, span: { start: offset, end: offset, line: 0, col: 0 } };
}

/** `digit(7)` as the handler and the values it runs with, as Clarity writes a handler call. */
export function handlerOf(event: Event, call: string): Handler {
  const open = call.indexOf("(");
  if (open < 0) return { event, handler: call.trim() };
  const handler = call.slice(0, open).trim();
  const inner = call.slice(open + 1, call.lastIndexOf(")"));
  return { event, handler, args: splitValues(inner).map(literalOf) };
}

/** The values between a call's brackets, split at the commas outside quotes. */
function splitValues(text: string): string[] {
  const values: string[] = [];
  let current = "";
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === "\\" && quoted) {
      current += c + (text[i + 1] ?? "");
      i++;
    } else if (c === '"') {
      quoted = !quoted;
      current += c;
    } else if (c === "," && !quoted) {
      values.push(current.trim());
      current = "";
    } else current += c;
  }
  if (current.trim()) values.push(current.trim());
  return values;
}

/** A value as Clarity writes it: a number, a quoted text, true or false, or a name. */
function literalOf(text: string): Literal {
  if (text.startsWith('"')) return { kind: "str", value: JSON.parse(text) as string };
  if (text === "true" || text === "false") return { kind: "bool", value: text === "true" };
  if (/^-?\d/.test(text)) return { kind: "number", value: text };
  return { kind: "ident", value: text };
}
