// The report tab's model of a `.report`: read, written back the same, and its edits.
import { test } from "node:test";
import assert from "node:assert/strict";
import { mm, newPart, paperSize, partLabel, partLook, readReport, stylesheetRules, withDeclaration, writeReport } from "./reportDesign.ts";

const INVOICE = `<Report name="Invoice" paper="A4" margin="20mm" stylesheet="invoice.css">
  <Data name="order" type="Order" />
  <PageHeader height="10mm">
    <Text at="0,0" size="80,6" key="shopName" />
  </PageHeader>
  <ReportHeader height="30mm">
    <Text at="0,0" size="80,10" key="invoiceTitle" class="title" />
    <Field at="120,0" size="60,6" bind="order.number" align="right" />
  </ReportHeader>
  <Detail height="7mm" list="order.lines">
    <Field at="0,0" size="100,6" bind="line.product" />
  </Detail>
  <ReportFooter height="20mm">
    <Sum at="150,0" size="30,6" of="line.price" />
    <Markdown at="0,8" size="180,10" bind="order.notes" />
  </ReportFooter>
  <PageFooter height="8mm">
    <PageNumber at="80,0" size="20,5" />
  </PageFooter>
</Report>
`;

test("a report reads into its paper, data, bands and parts, and writes back the same", () => {
  const model = readReport(INVOICE);
  assert.deepEqual(model.attrs, { name: "Invoice", paper: "A4", margin: "20mm", stylesheet: "invoice.css" });
  assert.deepEqual(model.data, [{ name: "order", type: "Order" }]);
  assert.deepEqual(model.bands.map((b) => [b.kind, b.height, b.parts.length]), [["PageHeader", 10, 1], ["ReportHeader", 30, 2], ["Detail", 7, 1], ["ReportFooter", 20, 2], ["PageFooter", 8, 1]]);
  assert.deepEqual(model.bands[2].attrs, { list: "order.lines" });
  assert.deepEqual(model.bands[1].parts[1], { kind: "Field", at: [120, 0], size: [60, 6], attrs: { bind: "order.number", align: "right" } });
  assert.equal(writeReport(model), INVOICE);
});

test("bands are written in the order they print, whatever order they were added in", () => {
  const model = readReport(`<Report name="R"><PageFooter height="8mm" /><ReportHeader height="20mm" /></Report>`);
  assert.match(writeReport(model), /ReportHeader[\s\S]*PageFooter/);
});

test("what is no report is said", () => {
  assert.throws(() => readReport("<Report"), /not closed/);
  assert.throws(() => readReport(`<Report><Pagefooter height="1mm" /></Report>`), /no 'Pagefooter'/);
  assert.throws(() => readReport(`<Bill />`), /starts with <Report>/);
});

test("paper, lengths, new parts and their labels", () => {
  assert.deepEqual(paperSize(readReport(`<Report paper="A5 landscape" />`)), [210, 148]);
  assert.equal(mm("2cm", 0), 20);
  assert.equal(mm("72pt", 0), 25.4);
  assert.deepEqual(newPart("Sum", [10, 0]).attrs, { of: "", align: "right" });
  assert.equal(partLabel({ kind: "Sum", at: [0, 0], size: [1, 1], attrs: { of: "line.price" } }), "Σ line.price");
  assert.equal(partLabel(newPart("PageNumber", [0, 0])), "#");
});

test("an attribute with no value is not written", () => {
  const model = readReport(`<Report name="R"><ReportHeader height="20mm"><Text at="0,0" size="10,5" key="a" /></ReportHeader></Report>`);
  model.bands[0].parts[0].attrs.align = undefined as unknown as string;
  assert.doesNotMatch(writeReport(model), /align/);
});

test("a part looks as its class and the body say", () => {
  const rules = stylesheetRules("/* c */ body { font-size: 9pt } .title, .big { font-size: 18pt; font-weight: bold; color: #1f5fbf }");
  assert.deepEqual([partLook(rules, "title").sizePt, partLook(rules, "title").bold, partLook(rules, "title").color], [18, true, "#1f5fbf"]);
  assert.equal(partLook(rules, undefined).sizePt, 9);
  assert.equal(partLook({}, "none").sizePt, 10.5);
});

test("a class's declaration is set, changed or taken out, the rest kept", () => {
  let css = "/* the shop's look */\nbody { font-size: 9pt }\n";
  css = withDeclaration(css, ".title", "font-size", "18pt");
  assert.equal(css, "/* the shop's look */\nbody { font-size: 9pt }\n.title { font-size: 18pt; }\n");
  css = withDeclaration(css, ".title", "color", "#1f5fbf");
  css = withDeclaration(css, ".title", "font-size", "20pt");
  assert.deepEqual(stylesheetRules(css)[".title"], { color: "#1f5fbf", "font-size": "20pt" });
  css = withDeclaration(css, ".title", "color", "");
  assert.deepEqual(stylesheetRules(css)[".title"], { "font-size": "20pt" });
  assert.match(css, /the shop's look/);
});
