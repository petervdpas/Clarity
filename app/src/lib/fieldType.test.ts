// A field's type as the entity editor shows it and writes it back.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ofChoices, typeParts, typeText, withKind } from "./fieldType.ts";

const names = { enums: ["Status"], records: ["Address"] };

test("a type text comes apart into its kind, what it holds and its sizes", () => {
  assert.deepEqual(typeParts("text(80)", names), { kind: "text", of: "", length: 80, precision: null, scale: null });
  assert.deepEqual(typeParts("decimal(10, 2)", names), { kind: "decimal", of: "", length: null, precision: 10, scale: 2 });
  assert.deepEqual(typeParts("ref Customer", names).of, "Customer");
  assert.deepEqual(typeParts("list<text>", names), { kind: "list", of: "text", length: null, precision: null, scale: null });
  assert.equal(typeParts("Status", names).kind, "enum");
  assert.equal(typeParts("Address", names).kind, "record");
  assert.equal(typeParts("money", names).kind, "money");
});

test("the parts are written back as the tools write them", () => {
  for (const text of ["text", "text(80)", "decimal(10, 2)", "ref Customer", "list<Order>", "Status", "Address", "secret", "folder"]) {
    assert.equal(typeText(typeParts(text, names)), text);
  }
  assert.equal(typeText({ kind: "decimal", of: "", length: null, precision: 10, scale: null }), null, "a decimal needs both sizes");
  assert.equal(typeText({ kind: "ref", of: "", length: null, precision: null, scale: null }), null);
});

test("a new kind starts where the tools' examples do, and offers what it can hold", () => {
  assert.equal(typeText(withKind("text", [])), "text(80)");
  assert.equal(typeText(withKind("decimal", [])), "decimal(10, 2)");
  const all = { ...names, entities: ["Customer"], simple: ["text", "integer"] };
  assert.deepEqual(ofChoices("list", all), ["Customer", "Address", "text", "integer"]);
  assert.equal(typeText(withKind("ref", ofChoices("ref", all))), "ref Customer");
  assert.deepEqual(ofChoices("money", all), []);
});
