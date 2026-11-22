// The designer's copies of what the spec decides about a window's parts.
// model.ts imports without extensions, as Vite does, so its lists are read as text.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
/** A field type's variant as the generated types name its kind: DateTime is dateTime. */
const kindOf = (variant: string) => variant[0].toLowerCase() + variant.slice(1);

test("a table cell edits the field types the spec's cell_edits names", () => {
  const model = read("./model.ts");
  const kinds = [...(model.match(/export const CELL_EDIT_KINDS = \[([^\]]*)\]/)?.[1] ?? "").matchAll(/"([A-Za-z]+)"/g)].map((m) => m[1]);
  const spec = read("../../../../crates/clarity-spec/src/widgets.rs");
  const start = spec.indexOf("pub fn cell_edits(");
  const named = [...spec.slice(start, spec.indexOf("\n}\n", start)).matchAll(/FieldType::([A-Za-z]+)/g)].map((m) => kindOf(m[1]));
  assert.ok(kinds.length > 0);
  assert.deepEqual(kinds.sort(), named.sort());
});
