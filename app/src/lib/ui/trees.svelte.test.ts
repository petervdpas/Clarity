// A tree on the design canvas, run through the real Table part: the
// sample records' references to their own kind hold the text of the row
// they point at, and the designer shows every level open.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import Table, { type Row } from "./Table.svelte";

function shown(target: HTMLElement): string[] {
  return [...target.querySelectorAll("tbody td")].map((td) => td.textContent?.trim() ?? "");
}

test("a tree in the designer shows all its levels, linked by the text sample references hold", () => {
  const target = document.createElement("div");
  const folders: Row[] = [
    { name: "Documents", parent: null },
    { name: "Music", parent: null },
    { name: "Invoices", parent: "Documents" },
    { name: "2026", parent: "Invoices" },
  ];
  const table = mount(Table, {
    target,
    props: {
      columns: [{ field: "name", label: "Name", kind: "text", edit: false }],
      rows: folders,
      design: true,
      header: false,
      tree: { parent: "parent", open: "Expand", close: "Collapse", key: "name" },
      onselect: () => {},
      onopen: () => {},
      onedit: () => {},
    },
  });
  flushSync();
  expect(shown(target)).toEqual(["Documents", "Invoices", "2026", "Music"]);
  const indents = [...target.querySelectorAll<HTMLElement>("tbody .indent")].map((s) => s.style.width);
  expect(indents).toEqual(["0px", "16px", "32px", "0px"]);
  unmount(table);
});
