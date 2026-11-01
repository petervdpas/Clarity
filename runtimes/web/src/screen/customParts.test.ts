import { test } from "node:test";
import assert from "node:assert/strict";
import { changedValues, covered, literalShown, paramsText, placeOf, shownValues, writtenValue } from "./customParts.ts";

test("a part's values are those written in the window, then those code set", () => {
  const written = { speed: { kind: "number" as const, value: "5" }, title: { kind: "str" as const, value: "@levelOne" }, running: { kind: "bool" as const, value: true } };
  const words = (text: string) => (text === "@levelOne" ? "Level one" : text);
  assert.deepEqual(shownValues(written, undefined, words), { speed: 5, title: "Level one", running: true });
  assert.deepEqual(shownValues(written, { speed: 8 }, words), { speed: 8, title: "Level one", running: true });
  assert.deepEqual(shownValues({ size: { kind: "number", value: "2.50" } }, undefined, words), { size: "2.50" });
  assert.deepEqual(shownValues({ mood: { kind: "ident", value: "Happy" } }, undefined, words), { mood: "Happy" });
});

test("only the values that changed are set again", () => {
  assert.deepEqual(changedValues(null, { speed: 5, title: "x" }), [["speed", 5], ["title", "x"]]);
  assert.deepEqual(changedValues({ speed: 5, title: "x" }, { speed: 6, title: "x" }), [["speed", 6]]);
  assert.deepEqual(changedValues({ list: [1, 2] }, { list: [1, 2] }), []);
});

test("a part is placed in the webview's pixels", () => {
  assert.deepEqual(placeOf({ left: 10.4, top: 20.6, width: 300.2, height: 199.5 }, 1), { x: 10, y: 21, width: 300, height: 200 });
  assert.deepEqual(placeOf({ left: 10, top: 20, width: 300, height: 200 }, 2), { x: 20, y: 40, width: 600, height: 400 });
});

test("a part under one of Clarity's overlays is covered", () => {
  const part = { left: 0, top: 0, width: 100, height: 100 };
  assert.equal(covered(part, []), false);
  assert.equal(covered(part, [{ left: 50, top: 50, width: 10, height: 10 }]), true);
  assert.equal(covered(part, [{ left: 100, top: 0, width: 10, height: 10 }]), false);
  assert.equal(covered(part, [{ left: 0, top: 0, width: 0, height: 0 }]), false);
});

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
