// What the report tab's designer draws with. The report's model, read and
// written, is the spec's (clarity_spec::report_design, pinned by its own cases),
// which the designer reaches through the backend.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { BANDS, PAPERS, PARTS, REPEATED, mm, paperSize, partLabel, partLook, type ReportModel } from "./reportDesign.ts";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
/** The words of a `pub const NAME: [&str; n] = [...]` of a Rust file. */
const words = (rust: string, name: string) => [...(rust.match(new RegExp(`pub const ${name}: \\[&str; \\d+\\] = \\[([^\\]]*)\\]`))?.[1] ?? "").matchAll(/"([A-Za-z0-9]+)"/g)].map((m) => m[1]);

test("paper, lengths and labels", () => {
  const report: ReportModel = { attrs: { paper: "A5 landscape" }, data: [], bands: [] };
  assert.deepEqual(paperSize(report), [210, 148]);
  assert.equal(mm("2cm", 0), 20);
  assert.equal(mm("72pt", 0), 25.4);
  assert.equal(partLabel({ kind: "Sum", at: [0, 0], size: [1, 1], attrs: { of: "line.price" } }), "Σ line.price");
  assert.equal(partLabel({ kind: "PageNumber", at: [0, 0], size: [10, 5], attrs: { align: "center" } }), "#");
});

test("a part looks as its class and the body say", () => {
  const rules = { body: { "font-size": "9pt" }, ".title": { "font-size": "18pt", "font-weight": "bold", color: "#1f5fbf" } };
  assert.deepEqual([partLook(rules, "title").sizePt, partLook(rules, "title").bold, partLook(rules, "title").color], [18, true, "#1f5fbf"]);
  assert.equal(partLook(rules, undefined).sizePt, 9);
  assert.equal(partLook({}, "none").sizePt, 10.5);
});

test("the bands, parts and papers the designer offers are the spec's", () => {
  const design = read("../../../crates/clarity-spec/src/report_design.rs");
  const reports = read("../../../crates/clarity-spec/src/reports.rs");
  assert.deepEqual([...BANDS], words(design, "PRINT_ORDER"));
  assert.deepEqual([...REPEATED], words(design, "REPEATED"));
  assert.deepEqual(Object.keys(PAPERS), words(design, "PAPERS"));
  assert.deepEqual([...PARTS].sort(), words(reports, "PARTS").sort());
});
