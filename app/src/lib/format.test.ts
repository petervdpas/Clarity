// The conformance cases every renderer must meet (conformance/formats.json),
// run against the web renderer's formatting. `npm test` runs them.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { editText, formatValue, parseNumber, userLocale, type FieldFormat } from "./format.ts";

type Shows = { what: string; locale: string; currency: string | null; field: FieldFormat; value: string | number; shows: string };
type Reads = { what: string; locale: string; currency: string | null; field: FieldFormat; typed: string; value: string | number | null };

const cases: { shows: Shows[]; reads: Reads[] } = JSON.parse(
  readFileSync(new URL("../../../conformance/formats.json", import.meta.url), "utf8"),
);

test("every value is shown as the conformance cases say", () => {
  assert.ok(cases.shows.length > 50);
  for (const c of cases.shows) {
    assert.equal(formatValue(c.field, c.value, { locale: c.locale, currency: c.currency }), c.shows, `${c.locale} ${c.what}`);
  }
});

test("typed text is read as the conformance cases say", () => {
  for (const c of cases.reads) {
    assert.equal(parseNumber(c.field, c.typed, { locale: c.locale, currency: c.currency }), c.value, `${c.locale} ${c.what}`);
  }
});

test("what a number input shows reads back as the same value", () => {
  const f = { locale: "nl-NL", currency: "EUR" };
  const money: FieldFormat = { kind: "money" };
  assert.equal(editText(money, "1250.5", f), "1.250,50");
  assert.equal(parseNumber(money, editText(money, "1250.5", f), f), "1250.5");
  const percent: FieldFormat = { kind: "decimal", scale: 4, format: "percent" };
  assert.equal(parseNumber(percent, editText(percent, "0.1234", f), f), "0.1234");
});

test("a stored value that is no number shows as it is", () => {
  assert.equal(formatValue({ kind: "decimal", scale: 2 }, "n/a", { locale: "en-US", currency: null }), "n/a");
  assert.equal(formatValue({ kind: "date" }, "someday", { locale: "en-US", currency: null }), "someday");
});

test("money without a currency keeps two decimals and no sign", () => {
  assert.equal(formatValue({ kind: "money" }, "3", { locale: "en-US", currency: null }), "3.00");
});

test("the locale is the system's, then the project's language, then English", () => {
  assert.equal(userLocale(["nl-NL", "en"], "de"), "nl-NL");
  assert.equal(userLocale([], "de-DE"), "de-DE");
  assert.equal(userLocale(undefined, null), "en");
});
