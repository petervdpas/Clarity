// How the inputs of a form behave, against the cases every renderer shares
// (conformance/forms.json), run through the real parts in a simulated page:
// ticks and picks are clicks and changes, typing goes into the inputs.

import { flushSync, mount, unmount, type Component } from "svelte";
import { expect, test } from "vitest";
import cases from "../../../../conformance/forms.json";
import { editText, parseNumber, type Formats } from "../format";
import type { JsonValue } from "../generated/serde_json/JsonValue";
import CheckInput from "./CheckInput.svelte";
import ChoiceInput, { type Choice } from "./ChoiceInput.svelte";
import DateInput from "./DateInput.svelte";
import DateTimeInput from "./DateTimeInput.svelte";
import NumberInput from "./NumberInput.svelte";
import RadioInput from "./RadioInput.svelte";
import TextInput from "./TextInput.svelte";

type Step = [string, ...JsonValue[]];
type Case = {
  name: string;
  steps: Step[];
  expect: {
    events: string[];
    values?: [string, JsonValue][];
    marked?: string[];
    shown?: [string, string][];
    choices?: Record<string, string[]>;
  };
};
type Field = (typeof cases.fields)[number];
type Task = { [field: string]: JsonValue };

const formats: Formats = { locale: "en-US", currency: null };
const fresh = (): Task => structuredClone(cases.record) as Task;
const choicesOf = (field: Field): Choice[] => ("choices" in field ? (field.choices as [JsonValue, string][]).map(([value, label]) => ({ value, label })) : []);

for (const c of cases.cases as Case[]) {
  test(c.name, () => {
    const state = $state({ record: fresh() });
    const events: string[] = [];
    const elsewhere = document.createElement("button");
    const mounted: ReturnType<typeof mount>[] = [];
    const holders: { [field: string]: HTMLElement } = {};

    for (const field of cases.fields) {
      const holder = document.createElement("div");
      document.body.append(holder);
      holders[field.field] = holder;
      const name = field.field;
      // The props every input takes, with `value` read from the record each time.
      const common = (extra: object = {}) => ({
        label: field.label,
        onchange: (next: JsonValue) => {
          state.record[name] = next;
          events.push(`change ${name}`);
        },
        onsubmit: () => events.push("submit"),
        ...extra,
        get value() {
          return state.record[name];
        },
      });
      const scale = "scale" in field ? field.scale : 0;
      const [part, props]: [Component<never>, object] = (() => {
        switch (field.kind) {
          case "text":
            return [TextInput, common()];
          case "memo":
            return [TextInput, common({ kind: "memo", onsubmit: undefined })];
          case "number":
            return [
              NumberInput,
              common({
                text: (value: JsonValue | undefined) => (typeof value === "string" || typeof value === "number" ? editText({ kind: "decimal", scale }, value, formats) : ""),
                read: (typed: string) => (typed.trim() === "" ? "0" : parseNumber({ kind: "decimal", scale }, typed, formats)),
              }),
            ];
          case "check":
          case "switch":
            return [
              CheckInput,
              {
                label: field.label,
                switch: field.kind === "switch",
                get checked() {
                  return state.record[name] === true;
                },
                onchange: common().onchange,
              },
            ];
          case "choice":
            return [ChoiceInput, common({ choices: choicesOf(field) })];
          case "radio":
            return [RadioInput, common({ name, choices: choicesOf(field) })];
          case "date":
            return [DateInput, common()];
          case "moment":
            return [
              DateTimeInput,
              common({
                onkeydown: (e: KeyboardEvent) => {
                  if (e.key === "Enter") events.push("submit");
                },
              }),
            ];
          default:
            throw new Error(`no part for ${field.kind}`);
        }
      })();
      mounted.push(mount(part, { target: holder, props: props as never }));
    }
    document.body.append(elsewhere);
    flushSync();

    const holder = (field: string) => holders[field] ?? fail(`no field ${field}`);
    const fail = (why: string): never => {
      throw new Error(`${c.name}: ${why}`);
    };
    /** The text field of an input: in a moment, its time. */
    const textOf = (field: string) =>
      holder(field).querySelector<HTMLInputElement>("input.time") ??
      holder(field).querySelector<HTMLInputElement>("textarea, input:not([type=checkbox]):not([type=radio]):not([type=date])") ??
      fail(`no text in ${field}`);
    let typedFrom: { input: HTMLInputElement; value: string } | null = null;

    for (const [what, ...args] of c.steps) {
      const field = args[0] as string;
      switch (what) {
        case "focus":
        case "type": {
          const input = textOf(field);
          input.focus();
          typedFrom = { input, value: input.value };
          if (what === "type") {
            input.value = args[1] as string;
            input.dispatchEvent(new Event("input", { bubbles: true }));
          }
          break;
        }
        // As a browser: Enter fires keydown; leaving a field fires change when its text changed.
        case "enter":
          document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
          break;
        case "leave":
          if (typedFrom && typedFrom.input.value !== typedFrom.value) typedFrom.input.dispatchEvent(new Event("change", { bubbles: true }));
          typedFrom = null;
          elsewhere.focus();
          break;
        case "tick":
          holder(field).querySelector<HTMLInputElement>("input[type=checkbox]")!.click();
          break;
        case "pick": {
          const spec = cases.fields.find((f) => f.field === field)!;
          const label = choicesOf(spec).find((ch) => ch.value === args[1])!.label;
          const select = holder(field).querySelector("select");
          if (select) {
            select.selectedIndex = [...select.options].findIndex((o) => o.text === label);
            select.dispatchEvent(new Event("change", { bubbles: true }));
          } else {
            const button = [...holder(field).querySelectorAll("label")].find((l) => l.textContent?.trim() === label)!.querySelector("input")!;
            button.click();
          }
          break;
        }
        case "pickDay": {
          const day = holder(field).querySelector<HTMLInputElement>("input[type=date]")!;
          day.value = args[1] as string;
          day.dispatchEvent(new Event("change", { bubbles: true }));
          break;
        }
        case "handler":
          state.record[field] = args[1];
          break;
        case "reload":
          state.record = fresh();
          break;
        default:
          fail(`unknown step ${what}`);
      }
      flushSync();
    }

    expect(events, "events").toEqual(c.expect.events);
    const wanted = fresh();
    for (const [field, value] of c.expect.values ?? []) wanted[field] = value;
    expect($state.snapshot(state.record), "values").toEqual(wanted);
    for (const field of c.expect.marked ?? []) expect(textOf(field).getAttribute("aria-invalid"), `${field} marked`).toBe("true");
    for (const [field, text] of c.expect.shown ?? []) {
      const select = holder(field).querySelector("select");
      const shown = select ? (select.selectedOptions[0]?.text ?? "") : textOf(field).value;
      expect(shown, `${field} shows`).toBe(text);
    }
    for (const [field, labels] of Object.entries(c.expect.choices ?? {})) {
      const select = holder(field).querySelector("select");
      const shown = select ? [...select.options].map((o) => o.text) : [...holder(field).querySelectorAll("label")].map((l) => l.textContent?.trim());
      expect(shown, `choices of ${field}`).toEqual(labels);
    }

    for (const part of mounted) unmount(part);
    for (const h of Object.values(holders)) h.remove();
    elsewhere.remove();
  });
}
