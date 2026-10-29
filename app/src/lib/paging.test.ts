// The paging form of a web API (DESIGN 5.23, paging): what each kind asks
// for and what a change of kind keeps.

import { test } from "node:test";
import assert from "node:assert/strict";

import { pagingFields, withKind } from "./paging.ts";
import type { Paging } from "./generated/Paging";

test("each kind starts as the project file's example writes it", () => {
  assert.deepEqual(withKind(null, "page"), { kind: "page", param: "page", size_param: "per_page", size: 100, next: "", items: "" });
  assert.deepEqual(withKind(null, "offset"), { kind: "offset", param: "offset", size_param: "limit", size: 100, next: "", items: "" });
  assert.deepEqual(withKind(null, "cursor"), { kind: "cursor", param: "after", size_param: "", size: 0, next: "meta.next", items: "" });
  assert.deepEqual(withKind(null, "link"), { kind: "link", param: "", size_param: "", size: 0, next: "", items: "" });
});

test("no kind means the API does not page", () => {
  const page: Paging = { kind: "page", param: "p", size_param: "n", size: 50, next: "", items: "data" };
  assert.equal(withKind(page, ""), null);
});

test("a change of kind keeps where the items are and a size both kinds have", () => {
  const page: Paging = { kind: "page", param: "p", size_param: "n", size: 50, next: "", items: "data" };
  assert.deepEqual(withKind(page, "offset"), { kind: "offset", param: "offset", size_param: "limit", size: 50, next: "", items: "data" });
  assert.deepEqual(withKind(page, "link"), { kind: "link", param: "", size_param: "", size: 0, next: "", items: "data" });
  assert.deepEqual(withKind(page, "page"), page);
});

test("each kind shows only the fields it writes", () => {
  assert.deepEqual(pagingFields("page"), { param: "Page parameter", size: true, next: false });
  assert.deepEqual(pagingFields("offset"), { param: "Offset parameter", size: true, next: false });
  assert.deepEqual(pagingFields("cursor"), { param: "Cursor parameter", size: false, next: true });
  assert.deepEqual(pagingFields("link"), { param: null, size: false, next: false });
});
