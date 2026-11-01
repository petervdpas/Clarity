import { test } from "node:test";
import assert from "node:assert/strict";
import { answerValues, answerable, numberFormat, type Ask } from "./asks.ts";

const lend: Ask = {
  question: { key: "lendTo" },
  form: { kind: "value", type: "record", of: "Friend", only: [3, 5], default: null },
  handler: "lend",
  values: [{ id: 7, title: "Blue" }],
};

test("a typed number is read in the format of its type; other values are no number", () => {
  assert.deepEqual(numberFormat("integer"), { kind: "integer" });
  assert.deepEqual(numberFormat("money"), { kind: "money" });
  assert.deepEqual(numberFormat("duration"), { kind: "duration" });
  assert.equal(numberFormat("text"), null);
  assert.equal(numberFormat("record"), null);
});

test("a choice needs one chosen; text may be empty; a yes needs nothing", () => {
  assert.equal(answerable(lend.form, null), false);
  assert.equal(answerable(lend.form, 3), true);
  assert.equal(answerable({ kind: "value", type: "text", of: null, only: null, default: null }, ""), true);
  assert.equal(answerable({ kind: "value", type: "integer", of: null, only: null, default: null }, null), false);
  assert.equal(answerable({ kind: "confirm", yes: null, no: null }, null), true);
});

test("an answer sends the values written for the handler, then the answer; a yes sends only the values", () => {
  assert.deepEqual(answerValues(lend, 5), [{ id: 7, title: "Blue" }, 5]);
  assert.deepEqual(answerValues({ ...lend, form: { kind: "confirm", yes: null, no: null } }, null), [{ id: 7, title: "Blue" }]);
});
