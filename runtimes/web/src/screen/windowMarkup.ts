// A window's markup as the parts the screen draws (DESIGN 7.8): the file
// Generate writes for each window (`ui/windows/Main.xml`), read when the
// application starts. Each element is a part with what differs from its
// kind's defaults (Rust's catalog, generated/part-defaults.json), written in
// Clarity's own words, as crates/clarity-gen/src/web_windows.rs writes it;
// each value is read by its field's type (generated/part-fields.json).

import partDefaults from "../generated/part-defaults.json";
import partFields from "../generated/part-fields.json";
import commandDefaults from "../generated/command-defaults.json";
import commandFields from "../generated/command-fields.json";
import type { Command } from "../generated/Command.ts";
import type { CommandProps } from "../generated/CommandProps.ts";
import type { Event } from "../generated/Event.ts";
import type { Handler } from "../generated/Handler.ts";
import type { Literal } from "../generated/Literal.ts";
import type { Menu } from "../generated/Menu.ts";
import type { Span } from "../generated/Span.ts";
import type { Widget } from "../generated/Widget.ts";
import type { WidgetPage } from "../generated/WidgetPage.ts";
import type { WidgetProps } from "../generated/WidgetProps.ts";

/** The words Clarity writes for a field of a part, where they differ from its name. */
const PART_WORDS: Record<string, string> = { key: "shortcut", taborder: "tabOrder", childList: "children", opens: "onClick" };
/** The same for a menu's item. */
const COMMAND_WORDS: Record<string, string> = { key: "shortcut", opens: "onClick", onclick: "handler" };
/** The elements that are a page of a container: a tab, a step, a section, a pane. */
const PAGES = ["Tab", "Step", "Section", "Pane"];
const EVENTS: readonly Event[] = ["click", "change", "enter", "leave", "select", "open", "pointer", "key", "close", "return", "start", "submit", "draw", "drag", "drop", "bend", "next", "finish", "reorder", "tick", "unconnected"];

type Kinds = Record<string, string>;

/** The parts of a window as its markup writes them; `first` is where their places start. */
export function windowFromMarkup(markup: string, first = 0): Widget[] {
  const document = new DOMParser().parseFromString(markup, "application/xml");
  const problem = document.getElementsByTagName("parsererror")[0];
  if (problem) throw new Error(`the window's markup is not well formed: ${problem.textContent ?? ""}`);
  let count = first;
  const place = (): Span => {
    const at = ++count;
    return { start: at, end: at, line: 0, col: 0 };
  };
  return [...document.documentElement.children].map((e) => partOf(e, place));
}

/** A value of a field, by its type: text as it is, a number, true or false, or JSON (a list, a record, or a name of the model's). */
function valueOf(kind: string | undefined, text: string): unknown {
  if (kind === "text") return text;
  if (kind === "number") return Number(text);
  if (kind === "bool") return text === "true";
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function partOf(element: Element, place: () => Span): Widget {
  const tag = element.tagName;
  const kind = tag[0].toLowerCase() + tag.slice(1);
  const defaults = (partDefaults as Record<string, WidgetProps>)[kind];
  if (!defaults) throw new Error(`a window's markup has no part <${tag}>`);
  const props = structuredClone(defaults) as WidgetProps & Record<string, unknown>;
  // A part one row high writes no `rows`.
  props.height = 1;
  const handlers: Handler[] = [];
  for (const { name: word, value } of [...element.attributes]) {
    if (word === "at") {
      const [col, row] = value.split(",");
      props.col = Number(col);
      props.row = Number(row);
    } else if (word === "span") props.width = Number(value);
    else if (word === "rows") props.height = Number(value);
    else if (word.startsWith("on") && (EVENTS as readonly string[]).includes(word.slice(2))) handlers.push(handlerOf(word.slice(2) as Event, value));
    // The fields its kind gives a value and this part has none of.
    else if (word === "none") for (const empty of value.split(" ").filter(Boolean)) props[PART_WORDS[empty] ?? empty] = null;
    else {
      const field = PART_WORDS[word] ?? word;
      props[field] = valueOf((partFields as Kinds)[field], value);
    }
  }
  props.handlers = handlers;
  const widget: Widget = { props, children: [], pages: [], menu: null, span: place() };
  for (const child of element.children) {
    if (child.tagName === "Menu") widget.menu = menuOf(child, place);
    else if (PAGES.includes(child.tagName)) widget.pages.push(pageOf(child, place));
    else widget.children.push(partOf(child, place));
  }
  return widget;
}

function pageOf(element: Element, place: () => Span): WidgetPage {
  const span = element.getAttribute("span");
  const page: WidgetPage = { title: element.getAttribute("title"), columns: span === null ? null : Number(span), children: [], span: place() };
  page.children = [...element.children].map((e) => partOf(e, place));
  return page;
}

function menuOf(element: Element, place: () => Span): Menu {
  const span = place();
  return { name: element.getAttribute("name"), items: [...element.children].map((e) => commandOf(e, place)), span };
}

function commandOf(element: Element, place: () => Span): Command {
  const props = structuredClone(commandDefaults) as CommandProps & Record<string, unknown>;
  for (const { name: word, value } of [...element.attributes]) {
    const field = COMMAND_WORDS[word] ?? word;
    props[field] = valueOf((commandFields as Kinds)[field], value);
  }
  const span = place();
  return { props, items: [...element.children].map((e) => commandOf(e, place)), span };
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
