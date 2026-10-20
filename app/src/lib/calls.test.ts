// A wired handler with its values, written as every stack writes it
// (conformance/calls.json): the renderer sends it as the handler to run.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

import { handlerCall, wiredCall } from "./calls.ts";
import type { Handler } from "./generated/Handler";

const cases: { cases: (Omit<Handler, "event"> & { text: string })[] } = JSON.parse(
  readFileSync(new URL("../../../conformance/calls.json", import.meta.url), "utf8"),
);

test("a wired call is written as every stack writes it", () => {
  assert.ok(cases.cases.length > 3);
  for (const c of cases.cases) assert.equal(handlerCall({ event: "click", handler: c.handler, args: c.args }), c.text);
});

test("an event sends the call wired to it, whether or not its part has a name", () => {
  // `button "7" on click: digit(7)`: a part without a name still runs its handler.
  const handlers: Handler[] = [
    { event: "click", handler: "digit", args: [{ kind: "number", value: "7" }] },
    { event: "change", handler: "changed" },
  ];
  assert.equal(wiredCall(handlers, "click"), "digit(7)");
  assert.equal(wiredCall(handlers, "change"), "changed");
  assert.equal(wiredCall(handlers, "select"), null);
});
