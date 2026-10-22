import { test } from "node:test";
import assert from "node:assert/strict";
import { boxName, marksOf, themeVariables } from "./diagramCanvas.ts";

test("the drawing takes the IDE theme's colors", () => {
  const vars: Record<string, string> = {
    "--bg": "#eef0f3",
    "--panel": "#ffffff",
    "--panel-alt": "#f6f7f9",
    "--border-strong": "#b7bec9",
    "--text": "#1d2330",
    "--text-muted": "#5d6677",
    "--accent": "#1f5fbf",
  };
  const theme = themeVariables((name) => vars[name] ?? "");
  assert.equal(theme.primaryColor, "#ffffff");
  assert.equal(theme.primaryTextColor, "#1d2330");
  assert.equal(theme.primaryBorderColor, "#b7bec9");
  assert.equal(theme.lineColor, "#5d6677");
  assert.equal(theme.attributeBackgroundColorEven, "#f6f7f9");
  assert.equal(theme.background, "#eef0f3");
});

test("a box is known by its id, its data-id, or the name it shows first", () => {
  const names = ["Order", "OrderLine", "Customer"];
  assert.equal(boxName({ id: "entity-OrderLine-3", dataId: null, text: "" }, names), "OrderLine");
  assert.equal(boxName({ id: "mermaid-7-classId-Customer-12", dataId: null, text: "" }, names), "Customer");
  assert.equal(boxName({ id: "x", dataId: "Order", text: "" }, names), "Order");
  assert.equal(boxName({ id: "", dataId: null, text: "OrderLine  text(80) product" }, names), "OrderLine");
  assert.equal(boxName({ id: "", dataId: null, text: "Unknown" }, names), null);
});

test("proposals mark the boxes they are about", () => {
  const marks = marksOf([
    { change: "add", subject: "Order" },
    { change: "add", subject: "Customer.email" },
    { change: "remove", subject: "Invoice.number" },
    { change: "change", subject: "Invoice.total" },
  ]);
  assert.deepEqual(marks, { Order: "add", Customer: "change", Invoice: "change" });
  assert.deepEqual(marksOf([{ change: "remove", subject: "OrderStatus.Approved" }]), { OrderStatus: "remove" });
});
