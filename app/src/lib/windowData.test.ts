// What a window holds, as the designer lists it and its parts bind it.

import { test } from "node:test";
import assert from "node:assert/strict";
import type { DataDecl } from "./generated/DataDecl.ts";
import { dataEntity, dataType, isList, startValue, translatedDefaults, valueField } from "./windowData.ts";

const span = { start: 0, end: 0, line: 1, col: 1 };
const ident = (name: string) => ({ name, span });
const record: DataDecl = { name: ident("customer"), of: { kind: "record", entity: ident("Customer") }, span };
const list: DataDecl = { name: ident("orders"), of: { kind: "list", entity: ident("Order") }, span };
const shown: DataDecl = { name: ident("shown"), of: { kind: "value", ty: { kind: { kind: "decimal" }, span }, default: null }, span };
const fresh: DataDecl = { name: ident("fresh"), of: { kind: "value", ty: { kind: { kind: "bool" }, span }, default: { kind: "bool", value: true } }, span };
const pending: DataDecl = { name: ident("pending"), of: { kind: "value", ty: { kind: { kind: "named", name: "Operation" }, span }, default: null }, span };
const word: DataDecl = { name: ident("word"), of: { kind: "value", ty: { kind: { kind: "text" }, span }, default: { kind: "str", value: "say \"hi\"" } }, span };

test("data is written as in the language", () => {
  assert.equal(dataType(record), "Customer");
  assert.equal(dataType(list), "list<Order>");
  assert.equal(dataType(shown), "decimal");
  assert.equal(dataType(fresh), "bool, default true");
  assert.equal(dataType(pending), "Operation");
  assert.equal(dataType(word), 'text, default "say \\"hi\\""');
});

test("a record and a list have an entity; a plain value has none", () => {
  assert.equal(dataEntity(record), "Customer");
  assert.equal(dataEntity(list), "Order");
  assert.equal(dataEntity(shown), null);
  assert.equal(isList(list), true);
  assert.equal(isList(record), false);
  assert.equal(isList(shown), false);
});

test("a part shows a plain value as a field named after it", () => {
  const field = valueField(shown)!;
  assert.equal(field.name.name, "shown");
  // A plain decimal keeps the digits it has: no scale of its own.
  assert.equal(field.type.kind, "decimal");
  assert.deepEqual(valueField(pending)!.type, { kind: "enum", name: { name: "Operation", span } });
  assert.deepEqual(valueField(fresh)!.type, { kind: "bool" });
  assert.equal(valueField(record), null);
});

test("a plain value starts at its default, or empty", () => {
  assert.equal(startValue(fresh, null), true);
  assert.equal(startValue(word, null), 'say "hi"');
  assert.equal(startValue(shown, null), "0");
  assert.equal(startValue(pending, "Add"), "Add");
  assert.equal(startValue({ ...shown, of: { kind: "value", ty: { kind: { kind: "integer" }, span }, default: { kind: "number", value: "2" } } }, null), 2);
});

test("a text value starts at its default in the person's language", () => {
  const nl: Record<string, string> = { 'say "hi"': "zeg hoi" };
  const tr = (text: string) => nl[text] ?? text;
  // Still at its default as written: shown translated.
  assert.deepEqual(translatedDefaults([word, fresh], { word: 'say "hi"', fresh: true }, tr), { word: "zeg hoi", fresh: true });
  // Changed by a handler: kept as it is.
  assert.deepEqual(translatedDefaults([word], { word: "bye" }, tr), { word: "bye" });
});
