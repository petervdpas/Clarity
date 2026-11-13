import { test } from "node:test";
import assert from "node:assert/strict";
import type { LibraryRow } from "@clarity/ui/generated/LibraryRow.ts";
import type { UsedFrom } from "@clarity/ui/generated/UsedFrom.ts";
import { USED_FROM, anyChosen, usedText } from "./libraries.ts";

function row(from: UsedFrom, more: Partial<LibraryRow> = {}): LibraryRow {
  return { key: "clarityUi", name: "Clarity.UI library", uses: "desktop programs", version: "0.43.0", chosen: null, from, folder: "/f", variable: null, choosable: true, ...more };
}

test("every source has its sentence", () => {
  const all: UsedFrom[] = ["chosen", "environment", "carried", "repository", "fetched", "toFetch", "unavailable", "published", "inProgram"];
  assert.deepEqual(Object.keys(USED_FROM).sort(), [...all].sort());
  for (const from of all) assert.ok(usedText(row(from, { variable: "CLARITY_UI" })).length > 0, from);
});

test("Clarity's own names the version that belongs to this Clarity", () => {
  assert.equal(usedText(row("toFetch")), "Clarity's own for 0.43.0, fetched on the first build that needs it.");
  assert.equal(usedText(row("published")), "Clarity's own for 0.43.0, which the stack's own tool fetches on the first build.");
});

test("a folder named by the environment says which variable", () => {
  assert.equal(usedText(row("environment", { variable: "CLARITY_RUNTIME" })), "The folder CLARITY_RUNTIME names where Clarity was started; it counts until a folder is chosen here.");
});

test("a factory reset has something to do only while a folder is chosen", () => {
  assert.equal(anyChosen([row("fetched"), row("published")]), false);
  assert.equal(anyChosen([row("fetched"), row("chosen", { chosen: "/mine" })]), true);
});
