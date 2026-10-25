// The conformance cases every renderer must meet (conformance/formats.json),
// run against the web renderer's formatting. `npm test` runs them.

import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import { editText, errorText, fillText, formatValue, numberEditing, parseNumber, userLocale, type FieldFormat, type ShownText } from "./format.ts";

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

type Fills = {
  what: string;
  locale: string;
  currency: string | null;
  text: ShownText;
  labels?: Record<string, Record<string, string>>;
  shows: string;
};

test("text with slots is filled as the conformance cases say", () => {
  const fills: Fills[] = (cases as unknown as { fills: Fills[] }).fills;
  assert.ok(fills.length >= 5);
  for (const c of fills) {
    const label = (enumName: string, value: string) => c.labels?.[enumName]?.[value] ?? value;
    assert.equal(fillText(c.text, { locale: c.locale, currency: c.currency }, label), c.shows, `${c.locale} ${c.what}`);
  }
});

test("plain text is shown as it is", () => {
  assert.equal(fillText("Saved {not a slot}", { locale: "en-US", currency: null }, (_, v) => v), "Saved {not a slot}");
});

test("a percent without a scale keeps its own digits", () => {
  assert.equal(formatValue({ kind: "decimal", format: "percent" }, "0.125", { locale: "en-US", currency: null }), "12.5%");
});

test("a number input shows its value for editing and reads what is typed; cleared it is zero", () => {
  const f = { locale: "en-US", currency: null };
  const hours = numberEditing({ kind: "decimal", scale: 1 }, f);
  assert.equal(hours.text("1234.5"), editText({ kind: "decimal", scale: 1 }, "1234.5", f));
  assert.equal(hours.text(null), "");
  assert.equal(hours.read("2.5"), "2.5");
  assert.equal(hours.read("lots"), null);
  assert.equal(hours.read(" "), "0");
  assert.equal(numberEditing({ kind: "integer" }, f).read(""), 0);
});

test("an error from the backend is shown as text: a refusal and a key keep their slots, anything else is its message", () => {
  const refused = { text: "Order {n} is over", values: { n: { kind: "text", value: "ORD-1" } } };
  assert.deepEqual(errorText(refused), refused);
  // Why the databases did not open, in Clarity's own words (conformance/unconnected.json).
  const reason = { key: "unconnectedFailed", values: { problem: { kind: "text", value: "the server did not answer" } } };
  assert.deepEqual(errorText(reason), reason);
  assert.deepEqual(errorText({ key: "unconnectedNotKept" }), { key: "unconnectedNotKept" });
  assert.equal(errorText("storage: locked"), "storage: locked");
  assert.equal(errorText(new Error("gone")), "Error: gone");
});
