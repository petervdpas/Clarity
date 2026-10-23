// A table shown a page at a time, as every renderer pages it (conformance/table-pages.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pageOf, pageWith } from "./ui/tableView.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/table-pages.json", import.meta.url), "utf8"));

test("a page shows the rows every renderer shows", () => {
  for (const c of cases.pages) {
    assert.deepEqual(pageOf(c.count, c.size, c.page), { page: c.shows, pages: c.pages, from: c.from, to: c.to, text: c.text }, JSON.stringify(c));
  }
});

test("a selected row shows its page", () => {
  for (const c of cases.selected) assert.equal(pageWith(c.size, c.at), c.page, JSON.stringify(c));
});
