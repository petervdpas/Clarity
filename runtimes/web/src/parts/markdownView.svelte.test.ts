// The markdown part on the web shows a table with its header, rows and
// columns' alignment, and math as its TeX, as Clarity.UI's markdown part does.

import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import MarkdownView from "./MarkdownView.svelte";

test("a table shows its header, rows and alignment, and math shows its TeX as code", () => {
  const target = document.createElement("div");
  const text = "| Item | Price |\n|:--|--:|\n| Paper | **4.50** |\n| Ink | 12.00 |\n\nArea $\\pi r^2$ here.\n\n$$E = mc^2$$";
  const shown = mount(MarkdownView, { target, props: { text, onlink: () => {} } });
  flushSync();
  const headers = [...target.querySelectorAll("th")].map((th) => [th.textContent, (th as HTMLElement).style.textAlign]);
  expect(headers).toEqual([
    ["Item", "left"],
    ["Price", "right"],
  ]);
  const rows = [...target.querySelectorAll("tbody tr")].map((tr) => [...tr.querySelectorAll("td")].map((td) => td.textContent));
  expect(rows).toEqual([
    ["Paper", "4.50"],
    ["Ink", "12.00"],
  ]);
  expect(target.querySelector("tbody td .bold")?.textContent).toBe("4.50");
  expect(target.querySelector("p .code")?.textContent).toBe("\\pi r^2");
  expect(target.querySelector("pre")?.textContent).toBe("E = mc^2");
  unmount(shown);
});
