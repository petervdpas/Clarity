// How Clarity writes a name, against the cases the spec runs (conformance/names.json).

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { camelCase, isCamelCase } from "./names.ts";
import { hyphenated } from "./commandLine.ts";

const cases = JSON.parse(readFileSync(new URL("../../../conformance/names.json", import.meta.url), "utf8"));

test("a name is camelCase as the cases say", () => {
  for (const c of cases.isCamelCase) assert.equal(isCamelCase(c.name), c.is, c.name);
});

test("words become camelCase as the cases say", () => {
  for (const c of cases.camelCase) assert.equal(camelCase(c.words), c.name, c.words);
});

test("a command is hyphenated as the cases say", () => {
  for (const c of cases.hyphenated) assert.equal(hyphenated(c.name), c.line, c.name);
});
