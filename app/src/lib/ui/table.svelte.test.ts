// How a table with edited columns behaves, against the cases every renderer
// shares (conformance/tables.json), run through the real Table part in a
// simulated page: clicks and ticks are clicks, typing goes into the inputs.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import cases from "../../../../conformance/tables.json";
import { editText, parseNumber, type Formats } from "../format";
import type { JsonValue } from "../generated/serde_json/JsonValue";
import Table, { type Row, type TableColumn } from "./Table.svelte";

type Step = [string, ...JsonValue[]];
type Case = {
  name: string;
  steps: Step[];
  expect: {
    events: string[];
    values?: [number, string, JsonValue][];
    marked?: [number, string][];
    shown?: [number, string, string][];
    choices?: Record<string, string[]>;
  };
};

const formats: Formats = { locale: "en-US", currency: null };

function columns(): TableColumn[] {
  return cases.columns.map((c) => ({
    field: c.field,
    label: c.label,
    kind: c.kind as TableColumn["kind"],
    edit: c.edit,
    switch: "switch" in c ? c.switch : false,
    choices: "choices" in c ? (c.choices as [JsonValue, string][]).map(([value, label]) => ({ value, label })) : undefined,
    number:
      c.kind === "number"
        ? {
            text: (value) => (typeof value === "string" || typeof value === "number" ? editText({ kind: "decimal", scale: c.scale }, value, formats) : ""),
            read: (text) => (text.trim() === "" ? "0" : parseNumber({ kind: "decimal", scale: c.scale }, text, formats)),
          }
        : undefined,
  }));
}

const fresh = (): Row[] => structuredClone(cases.rows) as Row[];

for (const c of cases.cases as Case[]) {
  test(c.name, () => {
    const state = $state({ rows: fresh(), selected: null as number | null });
    const events: string[] = [];
    const target = document.createElement("div");
    document.body.append(target);
    const elsewhere = document.createElement("button");
    document.body.append(elsewhere);
    const table = mount(Table, {
      target,
      props: {
        columns: columns(),
        get rows() {
          return state.rows;
        },
        get selected() {
          return state.selected;
        },
        onselect: (row: number) => {
          state.selected = row;
          events.push(`select ${row}`);
        },
        onopen: () => {},
        onedit: (row: number) => events.push(`change ${row}`),
      },
    });
    flushSync();

    const cell = (row: number, field: string) => {
      const td = target.querySelectorAll("tbody tr")[row]?.querySelector<HTMLElement>(`td[data-field="${field}"]`);
      if (!td) throw new Error(`${c.name}: no cell ${field} of row ${row}`);
      return td;
    };
    /** The text field of a cell: in a moment, its time. */
    const textOf = (row: number, field: string) =>
      (cell(row, field).querySelector<HTMLInputElement>("input.time") ?? cell(row, field).querySelector<HTMLInputElement>("input:not([type=checkbox]):not([type=date])"))!;
    let typedFrom: { input: HTMLInputElement; value: string } | null = null;

    for (const [what, ...args] of c.steps) {
      const row = args[0] as number;
      const field = args[1] as string;
      switch (what) {
        case "click":
          cell(row, "notes").click();
          break;
        case "tick":
          cell(row, field).querySelector<HTMLInputElement>("input[type=checkbox]")!.click();
          break;
        case "pick": {
          const select = cell(row, field).querySelector("select")!;
          const label = cases.columns.find((col) => col.field === field)!.choices!.find(([value]) => value === args[2])![1];
          select.selectedIndex = [...select.options].findIndex((o) => o.text === label);
          select.dispatchEvent(new Event("change", { bubbles: true }));
          break;
        }
        case "pickDay": {
          const day = cell(row, field).querySelector<HTMLInputElement>("input[type=date]")!;
          day.value = args[2] as string;
          day.dispatchEvent(new Event("change", { bubbles: true }));
          break;
        }
        case "type": {
          const input = textOf(row, field);
          input.focus();
          typedFrom = { input, value: input.value };
          input.value = args[2] as string;
          input.dispatchEvent(new Event("input", { bubbles: true }));
          break;
        }
        // As a browser: Enter in a text field fires keydown; leaving it fires change when its text changed.
        case "enter":
          document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
          break;
        case "leave":
          if (typedFrom && typedFrom.input.value !== typedFrom.value) typedFrom.input.dispatchEvent(new Event("change", { bubbles: true }));
          typedFrom = null;
          elsewhere.focus();
          break;
        case "handler":
          state.rows[row][field] = args[2];
          break;
        case "reload":
          state.rows = fresh();
          state.selected = null;
          break;
        default:
          throw new Error(`${c.name}: unknown step ${what}`);
      }
      flushSync();
    }

    expect(events, "events").toEqual(c.expect.events);
    const wanted = fresh();
    for (const [row, field, value] of c.expect.values ?? []) wanted[row][field] = value;
    expect($state.snapshot(state.rows), "values").toEqual(wanted);
    for (const [row, field] of c.expect.marked ?? []) expect(textOf(row, field).getAttribute("aria-invalid"), `${field} of row ${row} marked`).toBe("true");
    for (const [row, field, text] of c.expect.shown ?? []) {
      const td = cell(row, field);
      const input = td.classList.contains("edit") ? textOf(row, field) : null;
      expect(input ? input.value : td.textContent?.trim(), `${field} of row ${row} shows`).toBe(text);
    }
    for (const [field, labels] of Object.entries(c.expect.choices ?? {})) {
      expect([...cell(0, field).querySelectorAll("option")].map((o) => o.text), `choices of ${field}`).toEqual(labels);
    }

    unmount(table);
    target.remove();
    elsewhere.remove();
  });
}
