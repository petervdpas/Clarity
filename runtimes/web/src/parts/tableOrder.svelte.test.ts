// The order a table's rows show in, against the cases every renderer shares
// (conformance/table-order.json), run through the real Table part: a click on
// a header sorts, a click on a shown row chooses it.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import cases from "../../../../conformance/table-order.json";
import type { JsonValue } from "../generated/serde_json/JsonValue";
import Table, { type Row, type TableColumn } from "./Table.svelte";

type Case = {
  name: string;
  sortable: boolean;
  order: { field: string; desc: boolean } | null;
  steps: [string, JsonValue][];
  expect: { shown: number[]; events: string[]; chosen?: number | null };
};

function columns(): TableColumn[] {
  return cases.columns.map((c) => {
    const choices = "choices" in c ? (c.choices as [JsonValue, string][]).map(([value, label]) => ({ value, label })) : undefined;
    return {
      field: c.field,
      label: c.label,
      kind: c.kind as TableColumn["kind"],
      edit: false,
      choices,
      shown: choices ? (value) => choices.find((x) => x.value === (value ?? null))?.label ?? "" : undefined,
    };
  });
}

for (const c of cases.cases as Case[]) {
  test(c.name, () => {
    const state = $state({ selected: null as number | null });
    const events: string[] = [];
    const target = document.createElement("div");
    document.body.append(target);
    const table = mount(Table, {
      target,
      props: {
        columns: columns(),
        rows: structuredClone(cases.rows) as Row[],
        sortable: c.sortable,
        order: c.order,
        get selected() {
          return state.selected;
        },
        onselect: (row: number) => {
          state.selected = row;
          events.push(`select ${row}`);
        },
        onopen: () => {},
        onedit: () => {},
      },
    });
    flushSync();
    for (const [step, arg] of c.steps) {
      if (step === "sort") target.querySelector<HTMLElement>(`th[data-field="${arg}"]`)!.click();
      else if (step === "click") target.querySelectorAll<HTMLElement>("tbody tr")[arg as number].click();
      else throw new Error(`unknown step ${step}`);
      flushSync();
    }
    const shown = [...target.querySelectorAll<HTMLElement>("tbody tr")].map((tr) => Number(tr.dataset.row));
    expect(shown).toEqual(c.expect.shown);
    expect(events).toEqual(c.expect.events);
    if ("chosen" in c.expect) expect(state.selected).toBe(c.expect.chosen);
    unmount(table);
    target.remove();
  });
}
