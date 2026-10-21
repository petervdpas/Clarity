// The code behind a window, per element (as an IDE shows a component's
// event handlers): the handlers of the part picked in the designer, with the
// handlers they call, shown and edited on their own, and merged back into
// the window's code in place of what they were.

import type { Edit } from "./generated/Edit";
import type { Event as PartEvent } from "./generated/Event";
import type { CommandProps } from "./generated/CommandProps";
import type { WidgetProps } from "./generated/WidgetProps";

/** A handler of the code behind, with the comment lines above it. */
export type HandlerCode = {
  name: string;
  /** Its text, from its comment to its last line, ending in a newline. */
  text: string;
  /** Its text as written in the file, with the blank lines before it. */
  raw: string;
};

const HANDLER_LINE = /^handler\s+([A-Za-z_]\w*)/;

/** The handlers of `code`, in order. What comes before the first goes with it. */
export function handlersOf(code: string): HandlerCode[] {
  const lines = code.replace(/\n+$/, "").split("\n");
  const starts: number[] = [];
  lines.forEach((line, i) => {
    if (HANDLER_LINE.test(line)) {
      // Its comment: the comment lines right above it.
      let start = i;
      while (start > 0 && lines[start - 1].startsWith("//")) start--;
      starts.push(start);
    }
  });
  return starts.map((start, n) => {
    const from = n === 0 ? 0 : lastLine(lines, starts[n - 1], start) + 1;
    const to = lastLine(lines, start, n + 1 < starts.length ? starts[n + 1] : lines.length);
    const raw = lines.slice(from, to + 1).join("\n") + "\n";
    const own = lines.slice(start, to + 1);
    const name = HANDLER_LINE.exec(own.find((l) => HANDLER_LINE.test(l)) ?? "")?.[1] ?? "";
    return { name, text: own.join("\n") + "\n", raw };
  });
}

/** The last line before `end` that is not blank, from `start` on. */
function lastLine(lines: string[], start: number, end: number): number {
  let last = end - 1;
  while (last > start && lines[last].trim() === "") last--;
  return last;
}

/** The handlers `handler`'s code calls, of those in `names`. */
function calls(handler: HandlerCode, names: string[]): string[] {
  const body = handler.text.split("\n").filter((l) => !HANDLER_LINE.test(l));
  return names.filter((n) => n !== handler.name && body.some((l) => new RegExp(`(^|[^\\w.])${n}\\(`).test(l)));
}

/** A part's code: the handlers `roots` names, with the handlers they call, in the order the file has them. */
export function focusOf(code: string, roots: string[]): { names: string[]; text: string; firstLine: number } {
  const handlers = handlersOf(code);
  const all = handlers.map((h) => h.name);
  const names = new Set(roots.filter((r) => all.includes(r)));
  for (let grew = true; grew; ) {
    grew = false;
    for (const h of handlers) {
      if (!names.has(h.name)) continue;
      for (const called of calls(h, all)) {
        if (!names.has(called)) {
          names.add(called);
          grew = true;
        }
      }
    }
  }
  const focused = handlers.filter((h) => names.has(h.name));
  const first = handlers.findIndex((h) => names.has(h.name));
  return {
    names: focused.map((h) => h.name),
    text: focused.map((h) => h.text).join("\n"),
    firstLine: first < 0 ? 0 : lineOf(handlers, first),
  };
}

/** The line (from 1) where the text of handler `index` starts in the file. */
function lineOf(handlers: HandlerCode[], index: number): number {
  const before = handlers.slice(0, index).reduce((n, h) => n + h.raw.split("\n").length - 1, 0);
  const leading = handlers[index].raw.split("\n").length - handlers[index].text.split("\n").length;
  return before + leading + 1;
}

/**
 * The window's code with `edited` in place of the handlers `names` (one
 * block where the first of them was; at the end when there is none yet),
 * and the line where `edited` starts in it.
 */
export function mergeFocus(code: string, names: string[], edited: string): { text: string; firstLine: number } {
  const handlers = handlersOf(code);
  const block = edited.replace(/\s+$/, "");
  const at = handlers.findIndex((h) => names.includes(h.name));
  if (at < 0) {
    const rest = code.replace(/\s+$/, "");
    const text = rest === "" ? block + "\n" : `${rest}\n\n${block}\n`;
    return { text, firstLine: rest === "" ? 1 : rest.split("\n").length + 2 };
  }
  const parts: string[] = [];
  let firstLine = 1;
  handlers.forEach((h, i) => {
    if (i === at) {
      const so = parts.join("\n");
      firstLine = so === "" ? 1 : so.split("\n").length + 2;
      parts.push((i === 0 ? "" : "\n") + block);
    } else if (!names.includes(h.name)) {
      parts.push(h.raw.replace(/\n$/, ""));
    }
  });
  return { text: parts.join("\n") + "\n", firstLine };
}

/** The handler a wired event runs: `played` of `played(1)`. */
export function handlerName(wired: string): string {
  return wired.split("(")[0].trim();
}

/**
 * Wires `handler` to `event` of the part at `item` (events in catalog
 * order), or unwires the event with `null`. Wiring also writes the handler
 * in the code behind of `owner`, when it is not there yet.
 */
export function wireEvent(w: {
  item: number;
  props: WidgetProps;
  events: PartEvent[];
  owner: string;
  event: PartEvent;
  handler: string | null;
}): Edit {
  const others = w.props.handlers.filter((h) => h.event !== w.event);
  const handlers = w.handler ? [...others, { event: w.event, handler: w.handler }] : others;
  handlers.sort((a, b) => w.events.indexOf(a.event) - w.events.indexOf(b.event));
  // A click runs a handler or opens a window, not both.
  const onClick = w.event === "click" && w.handler ? null : w.props.onClick;
  const update: Edit = { op: "updateWidget", item: w.item, container: null, props: { ...w.props, handlers, onClick } };
  return w.handler ? { op: "batch", edits: [update, { op: "addHandler", owner: w.owner, name: w.handler }] } : update;
}

/** `words` as camelCase: `New game` and `new_game` are `newGame` (clarity_spec::names::camel_case). */
function camelCase(words: string): string {
  const parts = words.match(/[A-Za-z0-9]+/g) ?? [];
  const joined = parts.map((w, i) => (i === 0 ? w[0].toLowerCase() + w.slice(1) : w[0].toUpperCase() + w.slice(1))).join("");
  return /^[0-9]/.test(joined) ? "n" + joined : joined;
}

/** The handler a wired event gets: the part's name and the event, `saveClick`, `boardDraw`. */
export function eventHandler(part: string, event: string): string {
  return camelCase(`${part} ${event}`);
}

/** The handler a menu item, toolbar or ribbon button gets: after its name, or its caption, and `Click`. */
export function commandHandler(props: CommandProps): string {
  return camelCase(`${props.name ?? props.caption ?? "command"} click`);
}

/** What the built-in targets of `on click ->` do, by target. */
const BUILT_IN_ACTIONS: Record<string, string> = {
  quit: "quits the application",
  close: "closes the window",
  theme: "lets the person pick a theme",
  language: "lets the person pick a language",
};

/** What a command does instead of running code, when it opens a window or does a built-in thing; null when it runs a handler or nothing. */
export function commandAction(props: CommandProps): string | null {
  if (!props.onClick) return null;
  return BUILT_IN_ACTIONS[props.onClick] ?? `opens ${props.onClick}${props.newWindow ? " in a new window" : ""}`;
}

/** Wires `handler` to a command's click, writing it in the code behind of `owner`; null unwires it. A command runs code or opens a window, not both. */
export function wireCommand(w: { item: number; props: CommandProps; owner: string; handler: string | null }): Edit {
  const update: Edit = { op: "updateCommand", item: w.item, props: { ...w.props, handler: w.handler, onClick: w.handler ? null : w.props.onClick } };
  return w.handler ? { op: "batch", edits: [update, { op: "addHandler", owner: w.owner, name: w.handler }] } : update;
}

/** The first `keyword1`, `keyword2`... that `taken` does not have, as the designer names a new part. */
export function freePartName(taken: string[], keyword: string): string {
  let n = 1;
  while (taken.includes(keyword + n)) n++;
  return keyword + n;
}

/** A fix: an edit of the problem's file, or a command for the whole project. */
export type Fix = { label: string; edit: Edit } | { label: string; command: "make_texts_keys" };

/** The fixes the Problems list offers, by what the checker says (clarity_spec::names,
 *  the part checks and clarity_spec::translations). */
const FIXES: ({ says: string } & Fix)[] = [
  { says: "has no name; every", label: "Name them", edit: { op: "nameParts" } },
  { says: "is not camelCase:", label: "Make them camelCase", edit: { op: "camelNames" } },
  { says: "a sidebar's width is in pixels now", label: "Update the sidebars", edit: { op: "updateSidebars" } },
  { says: "is read by people, so it is a key", label: "Make texts keys", command: "make_texts_keys" },
  { says: "shows its name as its label; give it a key", label: "Make texts keys", command: "make_texts_keys" },
  { says: "shows a text without a key; give it one", label: "Make texts keys", command: "make_texts_keys" },
  { says: "so people read its name; give it a key", label: "Make texts keys", command: "make_texts_keys" },
  { says: "has no file; add translations/", label: "Make texts keys", command: "make_texts_keys" },
];

/** The fix for a problem the checker reports, for every such problem of its file or the project; null when there is none. */
export function fixOf(message: string): Fix | null {
  const fix = FIXES.find((f) => message.includes(f.says));
  if (!fix) return null;
  return "edit" in fix ? { label: fix.label, edit: fix.edit } : { label: fix.label, command: fix.command };
}
