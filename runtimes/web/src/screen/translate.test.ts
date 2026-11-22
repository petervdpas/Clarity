// Translations in the web renderer (I1c), against the shared cases in
// conformance/translations.json.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { keyValue, languageName, pickLanguage, translate, type Texts } from "./translate.ts";

type Pick = { what: string; locale: string; available: string[]; base: string | null; picks: string | null };
type KeyCase = { what: string; language: string | null; key: string; gives: string };
type Code = { model: { project: { locale: { language: string }; translations: Texts["values"]; version?: string } }; cases: KeyCase[] };
const cases: { picks: Pick[]; code: Code } = JSON.parse(readFileSync(new URL("../../../../conformance/translations.json", import.meta.url), "utf8"));

test("the language is picked as the conformance cases say", () => {
  for (const c of cases.picks) assert.equal(pickLanguage(c.locale, c.available, c.base), c.picks, c.what);
});

test("a key takes its value in the language picked, else the default's, as every runtime does", () => {
  const project = cases.code.model.project;
  for (const c of cases.code.cases) {
    const texts: Texts = { values: project.translations, picked: c.language, fallback: project.locale.language, version: project.version };
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

test("a language is named as the running app names it (Clarity.UI's choices.rs language_name)", () => {
  const rust = readFileSync(new URL("../../../rust-ui/src/choices.rs", import.meta.url), "utf8");
  const listed = rust.slice(rust.indexOf("const NAMES:"), rust.indexOf("];", rust.indexOf("const NAMES:")));
  const unescape = (text: string) => text.replace(/\\u\{([0-9a-f]+)\}/g, (_, hex) => String.fromCodePoint(Number.parseInt(hex, 16)));
  const pairs = [...listed.matchAll(/\("([a-z]+)", "([^"]*)"\)/g)];
  assert.ok(pairs.length > 20);
  for (const [, tag, name] of pairs) assert.equal(languageName(tag), unescape(name), tag);
  assert.equal(languageName("nl-BE"), "Nederlands", "a region keeps its language's name");
  assert.equal(languageName("xx"), "xx", "one not known is its tag");
});
