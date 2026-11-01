// A table shown a page at a time (conformance/table-pages.json), run through
// the real Table part: its bar turns the pages, a sort shows the first page,
// and a row selected by code shows its page.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import Table, { type Row, type TableColumn } from "./Table.svelte";

const columns: TableColumn[] = [{ field: "title", label: "Title", kind: "text", edit: false }];
const rows: Row[] = Array.from({ length: 7 }, (_, i) => ({ title: `Album ${String(i + 1).padStart(2, "0")}` }));

function shown(target: HTMLElement): string[] {
  return [...target.querySelectorAll("tbody td")].map((td) => td.textContent?.trim() ?? "");
}

test("the bar turns the pages and says which rows show", () => {
  const target = document.createElement("div");
  const state = $state({ selected: null as number | null });
  const table = mount(Table, {
    target,
    props: {
      columns,
      rows,
      page: 3,
      sortable: true,
      get selected() {
        return state.selected;
      },
      onselect: () => {},
      onopen: () => {},
      onedit: () => {},
    },
  });
  flushSync();
  expect(shown(target)).toEqual(["Album 01", "Album 02", "Album 03"]);
  expect(target.querySelector(".pager .shown")?.textContent).toBe("1-3 / 7");
  const [first, previous, next, last] = [...target.querySelectorAll<HTMLButtonElement>(".pager button")];
  expect(first.disabled && previous.disabled).toBe(true);
  next.click();
  flushSync();
  expect(shown(target)).toEqual(["Album 04", "Album 05", "Album 06"]);
  last.click();
  flushSync();
  expect(shown(target)).toEqual(["Album 07"]);
  expect(target.querySelector(".pager .shown")?.textContent).toBe("7-7 / 7");
  expect(next.disabled && last.disabled).toBe(true);
  // A sort shows the first page.
  target.querySelector<HTMLElement>("th")?.click();
  flushSync();
  expect(target.querySelector(".pager .shown")?.textContent).toBe("1-3 / 7");
  // A row selected by code shows its page.
  state.selected = 4;
  flushSync();
  expect(shown(target)).toEqual(["Album 04", "Album 05", "Album 06"]);
  unmount(table);
});
