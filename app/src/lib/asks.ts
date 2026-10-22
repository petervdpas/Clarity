// A question a handler asks, as the backend sends it in its reply
// (DESIGN 5.14): the renderer asks it in Clarity's own dialog when the
// handler has finished, and the answer runs the handler named, with the
// values written for it and, for a value, the answer last. The same JSON
// on every web stack.

import type { FieldFormat } from "./format";
import type { JsonValue } from "./generated/serde_json/JsonValue";
import type { ShownText } from "./format";

/** The kinds of value a question asks for. */
export type AskType = "text" | "integer" | "decimal" | "money" | "date" | "datetime" | "duration" | "enum" | "record";

/** What a question asks for: yes or no, or a value of a type (of an enum or entity), narrowed to `only`, starting at `default`. */
export type AskForm =
  | { kind: "confirm"; yes: ShownText | null; no: ShownText | null }
  | { kind: "value"; type: AskType; of: string | null; only: JsonValue[] | null; default: JsonValue | null };

export type Ask = { question: ShownText; form: AskForm; handler: string; values: JsonValue[] };

/** The format a typed number of `type` is read and shown in; null for a value that is no number. */
export function numberFormat(type: AskType): FieldFormat | null {
  switch (type) {
    case "integer":
    case "decimal":
    case "money":
    case "duration":
      return { kind: type };
    default:
      return null;
  }
}

/** Whether `value` answers the question: a choice or a number needs one; text may be empty; a yes needs nothing. */
export function answerable(form: AskForm, value: JsonValue | null): boolean {
  if (form.kind === "confirm") return true;
  if (form.type === "text") return true;
  return value !== null && value !== "";
}

/** What the answer sends: the values written for the handler, then the answer (none after a yes). */
export function answerValues(ask: Ask, answer: JsonValue | null): JsonValue[] {
  return ask.form.kind === "confirm" ? [...ask.values] : [...ask.values, answer];
}
