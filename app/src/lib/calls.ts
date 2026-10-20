// A wired handler with the values it runs with, written as the design
// writes it and as every stack writes it (conformance/calls.json):
// `digit(7)`, or the name alone without values. The renderer sends it as
// the handler to run; each backend dispatches on it.

import type { Handler } from "./generated/Handler";
import type { Literal } from "./generated/Literal";

function written(value: Literal): string {
  switch (value.kind) {
    case "number":
    case "ident":
      return value.value;
    case "bool":
      return String(value.value);
    case "str":
      return `"${value.value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n")}"`;
  }
}

export function handlerCall(h: Pick<Handler, "handler" | "args">): string {
  const args = h.args ?? [];
  return args.length === 0 ? h.handler : `${h.handler}(${args.map(written).join(", ")})`;
}

/**
 * The call a part's event sends, or null when nothing is wired to it. A part
 * needs no name for this: `button "7" on click: digit(7)` runs its handler.
 */
export function wiredCall(handlers: Handler[], event: Handler["event"]): string | null {
  const wired = handlers.find((h) => h.event === event);
  return wired ? handlerCall(wired) : null;
}
