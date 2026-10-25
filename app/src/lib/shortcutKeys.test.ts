// Keys set by the person, as every renderer sets them (conformance/shortcut-keys.json).

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { alsoHas, assign, refused, reset, type Command, type KeyChoices } from "./keys.ts";

type Cases = {
  commands: { id: string; layers: string[]; keys: string[] }[];
  assign: { name: string; choices: KeyChoices; id: string; key: string; expect: { choices: KeyChoices; swapped: string | null; also: string[] } }[];
  reset: { name: string; choices: KeyChoices; id: string; expect: KeyChoices }[];
  refused: [string, string | null][];
};
const cases: Cases = JSON.parse(readFileSync(new URL("../../../conformance/shortcut-keys.json", import.meta.url), "utf8"));
const commands: Command[] = cases.commands.map((c) => ({ ...c, name: c.id }));

for (const c of cases.assign) {
  test(`assign: ${c.name}`, () => {
    const done = assign(commands, c.choices, c.id, c.key);
    assert.deepEqual({ choices: done.choices, swapped: done.swapped, also: alsoHas(commands, c.choices, c.id, c.key) }, c.expect);
  });
}

for (const c of cases.reset) {
  test(`reset: ${c.name}`, () => assert.deepEqual(reset(commands, c.choices, c.id), c.expect));
}

test("refused keys", () => {
  for (const [key, why] of cases.refused) assert.equal(refused(key), why, key);
});
