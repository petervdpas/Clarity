// Translations in the web renderer (I1c), against the shared cases in
// conformance/translations.json.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { pickLanguage, translate, translateText } from "./translate.ts";

type Pick = { what: string; locale: string; available: string[]; base: string | null; picks: string | null };
const cases: { picks: Pick[] } = JSON.parse(readFileSync(new URL("../../../conformance/translations.json", import.meta.url), "utf8"));

test("the language is picked as the conformance cases say", () => {
  for (const c of cases.picks) assert.equal(pickLanguage(c.locale, c.available, c.base), c.picks, c.what);
});

test("a text shows its translation, or itself without one", () => {
  const nl = { "Customer card": "Klantkaart" };
  assert.equal(translate("Customer card", nl), "Klantkaart");
  assert.equal(translate("Save", nl), "Save");
  assert.equal(translate("Save", undefined), "Save");
});

test("text with slots is translated by its text as written, keeping its values", () => {
  const nl = { "Saved {customer.name}": "{customer.name} opgeslagen" };
  const values = { "customer.name": { kind: "text" as const, value: "Ada" } };
  assert.deepEqual(translateText({ text: "Saved {customer.name}", values }, nl), { text: "{customer.name} opgeslagen", values });
  assert.equal(translateText("Customer card", { "Customer card": "Klantkaart" }), "Klantkaart");
});
