import { test } from "node:test";
import assert from "node:assert/strict";
import { literalShown, paramsText, writtenValue } from "./customParts.ts";

test("a property's value is read as its type, or left out", () => {
  assert.deepEqual(writtenValue({ kind: "integer" }, "12"), { kind: "number", value: "12" });
  assert.equal(writtenValue({ kind: "integer" }, "1.5"), null);
  assert.equal(writtenValue({ kind: "integer" }, "fast"), null);
  assert.deepEqual(writtenValue({ kind: "decimal" }, "2.50"), { kind: "number", value: "2.50" });
  assert.deepEqual(writtenValue({ kind: "text" }, "Level one"), { kind: "str", value: "Level one" });
  assert.deepEqual(writtenValue({ kind: "bool" }, "true"), { kind: "bool", value: true });
  assert.deepEqual(writtenValue({ kind: "named", name: "Mood" }, "Happy"), { kind: "ident", value: "Happy" });
  assert.equal(writtenValue({ kind: "integer" }, ""), null);
  assert.equal(literalShown({ kind: "number", value: "12" }), "12");
  assert.equal(literalShown(undefined), "");
});

test("a tell's handler takes its values as written", () => {
  const ty = (kind: "integer" | "text") => ({ kind: { kind }, span: { start: 0, end: 0, line: 0, col: 0 } });
  const id = (name: string) => ({ name, span: { start: 0, end: 0, line: 0, col: 0 } });
  assert.equal(paramsText([{ name: id("points"), ty: ty("integer") }, { name: id("by"), ty: ty("text") }]), "points: integer, by: text");
  assert.equal(paramsText([]), "");
});
