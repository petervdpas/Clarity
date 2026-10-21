// Translations in the web renderer (I1c), against the shared cases in
// conformance/translations.json.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { keyValue, languageName, languagesOf, pickLanguage, rememberedChoice, translate, translateText, type Texts } from "./translate.ts";

type Pick = { what: string; locale: string; available: string[]; base: string | null; picks: string | null };
type KeyCase = { what: string; language: string | null; key: string; gives: string };
type Code = { model: { project: { locale: { language: string }; translations: Texts["values"] } }; cases: KeyCase[] };
const cases: { picks: Pick[]; code: Code } = JSON.parse(readFileSync(new URL("../../../conformance/translations.json", import.meta.url), "utf8"));

test("the language is picked as the conformance cases say", () => {
  for (const c of cases.picks) assert.equal(pickLanguage(c.locale, c.available, c.base), c.picks, c.what);
});

test("a key takes its value in the language picked, else the default's, as every runtime does", () => {
  const project = cases.code.model.project;
  for (const c of cases.code.cases) {
    const texts: Texts = { values: project.translations, picked: c.language, fallback: project.locale.language };
    // The renderer fills slots itself; the cases without slots check the lookup.
    if (!c.gives.includes("Ada")) assert.equal(keyValue(c.key, texts) ?? c.key, c.gives, c.what);
  }
});

test("a text of the project is a key's value, or written as it is", () => {
  const texts: Texts = { values: { en: { cardTitle: "Customer card" }, nl: { cardTitle: "Klantkaart" } }, picked: "nl", fallback: "en" };
  assert.equal(translate("@cardTitle", texts), "Klantkaart");
  assert.equal(translate("@cardTitle", { ...texts, picked: null }), "Customer card");
  assert.equal(translate("7", texts), "7");
  assert.equal(translate("@unknown", texts), "unknown");
});

test("a keyed text from the backend takes its value, keeping its values", () => {
  const texts: Texts = { values: { en: { savedName: "Saved {name}" }, nl: { savedName: "{name} opgeslagen" } }, picked: "nl", fallback: "en" };
  const values = { name: { kind: "text" as const, value: "Ada" } };
  assert.deepEqual(translateText({ key: "savedName", values }, texts), { text: "{name} opgeslagen", values });
  assert.deepEqual(translateText({ key: "save" }, texts), { text: "save", values: {} });
  assert.equal(translateText("As it is", texts), "As it is");
});

test("the language switch offers the base language first, each by its own name", () => {
  assert.deepEqual(languagesOf("en", ["nl", "de"]), ["en", "de", "nl"]);
  assert.deepEqual(languagesOf(null, ["en", "nl"]), ["en", "nl"]);
  assert.equal(languageName("nl"), "Nederlands");
  assert.equal(languageName("de"), "Deutsch");
});

test("a remembered choice applies while its language is there", () => {
  assert.equal(rememberedChoice("nl", "en", ["nl"]), "nl");
  // The base language shows the texts as written.
  assert.equal(rememberedChoice("en", "en", ["nl"]), null);
  // A language the project no longer has falls back to the locale.
  assert.equal(rememberedChoice("fr", "en", ["nl"]), undefined);
  assert.equal(rememberedChoice(null, "en", ["nl"]), undefined);
});
