// A wired handler with its values, written as every stack writes it
// (conformance/calls.json): the renderer sends it as the handler to run.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

import { handlerCall } from "./calls.ts";
import type { Handler } from "./generated/Handler";

const cases: { cases: (Omit<Handler, "event"> & { text: string })[] } = JSON.parse(
  readFileSync(new URL("../../../conformance/calls.json", import.meta.url), "utf8"),
);

test("a wired call is written as every stack writes it", () => {
  assert.ok(cases.cases.length > 3);
  for (const c of cases.cases) assert.equal(handlerCall({ event: "click", handler: c.handler, args: c.args }), c.text);
});
